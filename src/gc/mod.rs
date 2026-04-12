use std::collections::VecDeque;
use std::ops::Range;
use std::sync::{Mutex, RwLock};
use std::vec;

pub use segment::{Seg, Segment, LargeSegment};
use unsafe_ref::UnsafeRef;
use handle_table::HandleTable;
use crate::gcinterface::{GCToCLR, IGCToCLR, ScanFlags, SuspendReason};
use crate::objects::{HandleType, ObjectRef};

mod handle_table;
mod segment;
mod unsafe_ref;

pub struct RustGc {
    pub clr: GCToCLR,
    pub handle_table: RwLock<HandleTable>,
    segments: RwLock<Vec<UnsafeRef<dyn Seg>>>,
    finalization_queue: Mutex<VecDeque<ObjectRef>>,
}

impl RustGc {
    pub fn new(clr: *const IGCToCLR) -> RustGc {
        RustGc {
            clr: GCToCLR::new(clr),
            handle_table: RwLock::new(HandleTable::new()),
            segments: RwLock::new(vec![]),
            finalization_queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn add_segment(&mut self, size: usize) -> Range<*const usize> {
        let new_seg : Box<dyn Seg> =
            if size <= Segment::SIZE {
                Segment::new_boxed()
            } else {
                Box::new(LargeSegment::new(size))
            };
        let mut w = self.segments.write().unwrap();
        let range = new_seg.data().as_ptr_range();
        w.push(UnsafeRef::new(new_seg));
        range
    }

    pub fn complete_segment(&mut self, segment_end: usize) {
        let r = self.segments.read().unwrap();
        let Some(segment) = r.iter().find(|s| { s.data().as_ptr_range().end as usize == segment_end }) else { return };
        segment.get_mut().set_alloc_completed();
    }

    pub fn do_collect(&mut self, generation: i32) {
        println!("GC triggered for generation {}", generation);
        
        println!("Suspending EE");
        self.clr.suspend_ee(SuspendReason::GC);

        self.clr.gc_start_work(generation, 2);

        let r = self.segments.read().unwrap();
        let find_segment = |or: ObjectRef| r.iter().find(|s| s.contains(or));

        let mark_object = |or: ObjectRef| {
            let segment = find_segment(or).ok_or(())?;
            segment.get_mut().mark_object(or)
        };

        let is_object_dead = |or: ObjectRef|
            !or.is_null() && find_segment(or).is_some_and(|seg| !seg.is_marked(or).unwrap());

        let mut mark_queue : VecDeque<ObjectRef> = VecDeque::new();
        let try_mark_push = |mark_queue: &mut VecDeque<ObjectRef>, or: ObjectRef| {
            if mark_object(or).unwrap_or(false) {
                mark_queue.push_back(or);
            }
        };

        let mut handle_table_lock = self.handle_table.write().unwrap();

        // Start mark phase
        self.clr.scan_roots(generation, 2, true, false, false,
            |pp_obj, _sc, f| {
                let or =
                    if (*pp_obj).is_null() {
                        None
                    } else if f.contains(ScanFlags::MayBeInterior) {
                        find_segment(*pp_obj).and_then(|s| s.find_object(*pp_obj))
                    } else {
                        Some(*pp_obj)
                    };

                if let Some(obj) = or {
                    try_mark_push(&mut mark_queue, obj);
                }
            });
        let stack_roots = mark_queue.len();
        println!("Encountered {} roots from stack.", mark_queue.len());

        for h in handle_table_lock.iter().filter(|h| !h.object.is_null()) {
            match h.handle_type {
                HandleType::Strong | HandleType::Pinned => try_mark_push(&mut mark_queue, h.object),
                _ => {},
            }
        }
        println!("Encountered {} roots from handle.", mark_queue.len() - stack_roots);

        self.finalization_queue.lock().unwrap().iter()
            .for_each(|f| try_mark_push(&mut mark_queue, *f));

        while let Some(or) = mark_queue.pop_front() {
            let obj = unsafe { &mut * or };
                obj.for_each_obj_ref(|r| try_mark_push(&mut mark_queue, *r));
        }

        for h in handle_table_lock.iter_mut() {
            if h.handle_type == HandleType::Short && is_object_dead(h.object) {
                h.object = std::ptr::null_mut();
            }
        }

        let has_finalizable;

        // Mark finalizables
        {
            let mut finalizables: VecDeque<ObjectRef> = VecDeque::new();
            for seg in r.iter() {
                seg.get_mut().for_each_obj_mut(&mut |seg, or| {
                    let obj = unsafe { &mut *or };
                    if !seg.is_marked(or).unwrap() && obj.needs_finalization() && !seg.get_finalization_pending(or).unwrap() {
                        finalizables.push_back(or);
                        seg.set_finalization_pending(or, true).unwrap();
                        try_mark_push(&mut mark_queue, or);
                    }
                });
            }

            // Dependent handle target are treated similar to fields
            for h in handle_table_lock.iter_mut().filter(|h| h.handle_type == HandleType::Dependent) {
                if h.object.is_null() || is_object_dead(h.object) {
                    h.extra_or_secondary = 0;
                } else {
                    try_mark_push(&mut mark_queue, h.extra_or_secondary as ObjectRef);
                }
            };

            while let Some(or) = mark_queue.pop_front() {
                let obj = unsafe { &mut * or };
                obj.for_each_obj_ref(|r| try_mark_push(&mut mark_queue, *r));
            }

            let mut q = self.finalization_queue.lock().unwrap();
            println!("Find {} new objects eligible for finalization. Existing in queue: {}", finalizables.len(), q.len());
            q.append(&mut finalizables);
            has_finalizable = !q.is_empty();
        }

        for h in handle_table_lock.iter_mut() {
            if h.handle_type == HandleType::ShortRecurrsion && is_object_dead(h.object) {
                h.object = std::ptr::null_mut();
            }
        }

        drop(r);

        let mut heap_count = 0;
        let mut heap_bytes = 0;
        let mut marked_count = 0;
        let mut field_count = 0;
        let mut non_null_field_count = 0;
        {
            let r = self.segments.read().unwrap();
            for seg in r.iter() {
                seg.for_each_obj(&mut |or| {
                    unsafe {
                        // println!("Walking at {:016x}, MethodTable: {:016x}", or as usize, (*or).method_table as usize);
                        // println!("Object: HasComponentSize: {}, TotalSize: {}", (*or).has_component_size(), (*or).total_size());
                        heap_count += 1;
                        heap_bytes += (*or).total_size();
                        if seg.is_marked(or).unwrap_or(false) { marked_count += 1; }

                        (*or).for_each_obj_ref(|field| {
                            field_count += 1;
                            if !field.is_null() {
                                // println!("Non-null field at {:016x}, target: {:016x}, target MT: {:016x}", field as *const ObjectRef as usize, (*field) as usize, (**field).method_table as usize);
                                non_null_field_count += 1;
                            }
                        });
                    }
                });
            }
        }
        println!("Encountered totally {} objects on heap. Total size: {} bytes. Marked: {}.", heap_count, heap_bytes, marked_count);
        println!("Encountered totally {} fields on heap. Not null: {}.", field_count, non_null_field_count);

        // Start sweep phase
        {
            let mut w = self.segments.write().unwrap();
            let mut incomplete = 0;
            let mut i = 0;
            while i < w.len() {
                let seg = &mut w[i];
                if !seg.get_alloc_completed() {
                    incomplete += 1;
                }
                if seg.sweep() {
                    seg.clear_mark();
                    i += 1;
                } else {
                    println!("Removing empty segment at {:016x}", seg.data().as_ptr() as usize);
                    w.remove(i);
                }
            }
            println!("Segments ineligible for compact: {}", incomplete);
        }
        
        let mut heap_count = 0;
        let mut heap_bytes = 0;
        {
            let r = self.segments.read().unwrap();
            for seg in r.iter() {
                seg.for_each_obj(&mut |or| {
                    unsafe {
                        heap_count += 1;
                        heap_bytes += (*or).total_size();
                    }
                });
            }
        }
        println!("{} object survived after sweeping. Total size: {}", heap_count, heap_bytes);

        println!("Resuming EE");
        self.clr.gc_done(generation);
        self.clr.restart_ee(true);
        self.clr.enable_finalization(has_finalizable);
    }

    pub fn pop_finalizable(&mut self) -> Option<ObjectRef> {
        self.finalization_queue.lock().unwrap().pop_front()
    }

    pub fn reregister_finalization(&mut self, or: ObjectRef) {
        let r = self.segments.read().unwrap();
        let Some(seg) = r.iter().find(|seg| seg.contains(or)) else { return };
        unsafe { &mut *or }.set_finalizer_run(false);
        seg.get_mut().set_finalization_pending(or, false).unwrap();
    }
}
