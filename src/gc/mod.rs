mod handle_table;
mod segment;
mod unsafe_ref;

use std::collections::VecDeque;
use std::ops::Range;
use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::{Mutex, RwLock};
use std::vec;

use crate::gcinterface::*;
use crate::objects::*;
use handle_table::HandleTable;
pub use segment::*;
use unsafe_ref::UnsafeRef;

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

    pub fn add_segment(&mut self, size: usize, pin: bool) -> Range<*const usize> {
        let new_seg: Box<dyn Seg> =
            if pin || size >= Segment::SIZE {
                Box::new(LargeSegment::new(size))
            } else {
                Segment::new_boxed() 
            };
        let mut w = self.segments.write().unwrap();
        let range = new_seg.data().as_ptr_range();
        w.push(UnsafeRef::new(new_seg));
        range
    }

    pub fn do_collect(&mut self, generation: i32) {
        println!("GC triggered for generation {}", generation);

        println!("Suspending EE");
        self.clr.suspend_ee(SuspendReason::GC);

        self.clr.gc_start_work(generation, 2);

        let r = self.segments.read().unwrap();
        let find_segment = |or: ObjectRef| r.iter().find(|s| s.contains(or));

        let mark_object = |or: ObjectRef, pin: bool| {
            let segment = find_segment(or).ok_or(())?;
            segment.get_mut().mark_object(or, pin)
        };

        let is_object_dead = |or: ObjectRef|
            !or.is_null() && find_segment(or).is_some_and(|seg| !seg.is_marked(or).unwrap());

        let mut mark_queue: VecDeque<ObjectRef> = VecDeque::new();
        let try_mark_push = |mark_queue: &mut VecDeque<ObjectRef>, or: ObjectRef, pin: bool| {
            if mark_object(or, pin).unwrap_or(false) {
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
                    try_mark_push(&mut mark_queue, obj, f.contains(ScanFlags::Pinned));
                }
            });
        // println!("Encountered {} roots from stack.", mark_queue.len());

        for h in handle_table_lock.iter().filter(|h| !h.object.is_null()) {
            match h.handle_type {
                HandleType::Strong => try_mark_push(&mut mark_queue, h.object, false),
                HandleType::Pinned => try_mark_push(&mut mark_queue, h.object, true),
                _ => {}
            }
        }
        // println!("Encountered {} roots from stack & handle.", mark_queue.len() - stack_roots);

        for f in self.finalization_queue.lock().unwrap().iter() {
            try_mark_push(&mut mark_queue, *f, false);
        }

        while let Some(or) = mark_queue.pop_front() {
            let obj = unsafe { &mut *or };
            obj.for_each_obj_ref(|r| try_mark_push(&mut mark_queue, *r, false));
        }

        for h in handle_table_lock.iter_mut() {
            if h.handle_type == HandleType::Short && is_object_dead(h.object) {
                h.object = null_mut();
            }
        }

        let has_finalizable;

        // Mark finalizables
        {
            let mut finalizables: VecDeque<ObjectRef> = VecDeque::new();
            for seg in r.iter() {
                for or in seg.iter() {
                    let obj = unsafe { &mut *or };
                    if !seg.is_marked(or).unwrap() && obj.needs_finalization() && !seg.get_finalization_pending(or).unwrap() {
                        finalizables.push_back(or);
                        seg.get_mut().set_finalization_pending(or, true).unwrap();
                        try_mark_push(&mut mark_queue, or, false);
                    }
                }
            }

            // Dependent handle target are treated similar to fields
            for h in handle_table_lock.iter_mut().filter(|h| h.handle_type == HandleType::Dependent) {
                if h.object.is_null() || is_object_dead(h.object) {
                    h.extra_or_secondary = 0;
                } else {
                    try_mark_push(&mut mark_queue, h.extra_or_secondary as ObjectRef, false);
                }
            }

            while let Some(or) = mark_queue.pop_front() {
                let obj = unsafe { &mut *or };
                obj.for_each_obj_ref(|r| try_mark_push(&mut mark_queue, *r, false));
            }

            let mut q = self.finalization_queue.lock().unwrap();
            // println!("Find {} new objects eligible for finalization. Existing in queue: {}", finalizables.len(), q.len());
            q.extend(finalizables);
            has_finalizable = !q.is_empty();
        }

        for h in handle_table_lock.iter_mut() {
            if h.handle_type == HandleType::ShortRecurrsion && is_object_dead(h.object) {
                h.object = null_mut();
            }
        }

        drop(r);

        let mut heap_count = 0;
        let mut heap_bytes = 0;
        let mut marked_count = 0;
        let mut pinned_count = 0;
        let mut field_count = 0;
        let mut non_null_field_count = 0;
        {
            let r = self.segments.read().unwrap();
            for seg in r.iter() {
                for or in seg.iter() {
                    unsafe {
                        // println!("Walking at {:016x}, MethodTable: {:016x}", or as usize, (*or).method_table as usize);
                        // println!("Object: HasComponentSize: {}, TotalSize: {}", (*or).has_component_size(), (*or).total_size());
                        heap_count += 1;
                        heap_bytes += (*or).total_size_aligned();
                        if seg.is_marked(or).unwrap_or(false) { marked_count += 1; }
                        if seg.is_pinned(or).unwrap_or(false) { pinned_count += 1; }

                        (*or).for_each_obj_ref(|field| {
                            field_count += 1;
                            if !field.is_null() {
                                // println!("Non-null field at {:016x}, target: {:016x}, target MT: {:016x}", field as *const ObjectRef as usize, (*field) as usize, (**field).method_table as usize);
                                non_null_field_count += 1;
                            }
                        });
                    }
                }
            }
        }
        println!("Encountered totally {} objects on heap. Total size: {} bytes. Marked: {}. Pinned: {}.", heap_count, heap_bytes, marked_count, pinned_count);
        // println!("Encountered totally {} fields on heap. Not null: {}.", field_count, non_null_field_count);

        // Start sweep phase
        {
            let mut w = self.segments.write().unwrap();

            self.clr.for_each_alloc_context(|c| {
                if c.alloc_limit != 0 {
                    w.iter_mut()
                        .find(|s| s.data().as_ptr_range().end == c.alloc_limit as *const usize)
                        .unwrap()
                        .set_in_use();
                }
            });

            let mut i = 0;
            while i < w.len() {
                let seg = &mut w[i];
                if seg.get_in_use() {
                    i += 1;
                    continue;
                }

                if seg.sweep() {
                    i += 1;
                } else {
                    println!("Removing empty segment at {:016x}", seg.data().as_ptr() as usize);
                    w.remove(i);
                }
            }

            let heap_count = w.iter().flat_map(|s| s.iter()).count();
            let heap_bytes = w.iter().flat_map(|s| s.iter().map(|or| unsafe { (*or).total_size_aligned() })).sum::<usize>();
            // println!("{} object survived after sweeping. Total size: {}", heap_count, heap_bytes);

            const COMPACT_THRESHOLD: usize = Segment::SIZE / 4;
            let dropped_segments: Vec<_> = w
                .extract_if(.., |s|
                    !s.get_in_use() && !s.contains_pinned() && s.alive_bytes() < COMPACT_THRESHOLD)
                .collect();
            if dropped_segments.len() > 1 {
                // Compact small segments
                let mut destination = Segment::new_boxed();
                let mut data = destination.available_space_with_header();
                for seg in dropped_segments.iter() {
                    for or in seg.iter() {
                        let ptr_size = unsafe { (*or).total_size_aligned() / size_of::<usize>() };
                        if data.len() < ptr_size {
                            w.push(UnsafeRef::new(destination));
                            destination = Segment::new_boxed();
                            data = destination.available_space_with_header();
                        }

                        let src = unsafe { &*slice_from_raw_parts((or as *const usize).wrapping_sub(1), ptr_size) };
                        data[..ptr_size].copy_from_slice(src);
                        unsafe {
                            *(or as *mut usize).wrapping_sub(1) = &raw const data[1] as usize;
                        }
                        // println!("Copied object sized {} from {:016x} to {:016x}", ptr_size * size_of::<usize>(), or as usize, &raw const data[1] as usize);

                        data = &mut data[ptr_size..];
                    }
                    println!("Dropping moved segment at {:016x}", seg.data().as_ptr() as usize);
                }
                w.push(UnsafeRef::new(destination));

                // Modify moved references
                let fix_ref = |pp_obj: &mut ObjectRef| {
                    if dropped_segments.iter().any(|s| s.contains(*pp_obj)) {
                        let move_target = unsafe { *(*pp_obj as *mut usize).wrapping_sub(1) };
                        // println!("Fixing reference at {:016x} from {:016x} to {:016x}", pp_obj as *const ObjectRef as usize, *pp_obj as usize, move_target);
                        *pp_obj = move_target as ObjectRef;
                    }
                };
                let fix_ref_interior = |pp_obj: &mut ObjectRef| {
                    let Some(or) = dropped_segments.iter().find_map(|s| s.find_object(*pp_obj)) else { return };
                    unsafe {
                        let move_target = *(or as *mut usize).wrapping_sub(1);
                        let offset = (*pp_obj).byte_offset_from_unsigned(or);
                        // println!("Fixing interior reference at {:016x} from {:016x} to {:016x}, offset={}", pp_obj as *const ObjectRef as usize, *pp_obj as usize, move_target + offset, offset);
                        *pp_obj = (move_target + offset) as ObjectRef;
                    }
                };

                self.clr.scan_roots(generation, 2, false, false, false,
                    |pp_obj, _, f| {
                        if f.contains(ScanFlags::MayBeInterior) {
                            fix_ref_interior(pp_obj);
                        } else {
                            fix_ref(pp_obj);
                        }
                    });
                for seg in w.iter() {
                    for or in seg.iter() {
                        let obj = unsafe { &mut *or };
                        obj.for_each_obj_ref(fix_ref);
                    }
                }
                for h in handle_table_lock.iter_mut() {
                    fix_ref(&mut h.object);
                    if h.handle_type == HandleType::Dependent {
                        unsafe {
                            fix_ref(std::mem::transmute(&mut h.extra_or_secondary));
                        }
                    }
                }
            } else {
                w.extend(dropped_segments);
            }

            for seg in w.iter_mut() {
                seg.clear_flags();
            }

            let heap_count_after = w.iter().flat_map(|s| s.iter()).count();
            let heap_bytes_after = w.iter().flat_map(|s| s.iter().map(|or| unsafe { (*or).total_size_aligned() })).sum::<usize>();
            assert_eq!(heap_count, heap_count_after);
            assert_eq!(heap_bytes, heap_bytes_after);
        }

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
