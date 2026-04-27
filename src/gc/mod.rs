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

        // All threads touching managed references must be paused at least for root marking and object moving.
        // As a naive implementation without concurrent collection, do a simple STW during the entire GC.
        // The CLR threading model ensures that every thread is suspended at safe point where every manipulated
        // reference can be precisely reported.
        println!("Suspending EE");
        self.clr.suspend_ee(SuspendReason::GC);

        self.clr.gc_start_work(generation, 2);

        let r = self.segments.read().unwrap();
        let find_segment = |or: ObjectRef| r.iter().find(|s| s.contains(or));

        let is_object_dead = |or: ObjectRef|
            or.is_null() || find_segment(or).is_some_and(|seg| !seg.is_marked(or).unwrap());

        let mut mark_queue: VecDeque<ObjectRef> = VecDeque::new();
        // Mark an object as reachable. If the object is newly marked, add it into the queue for field walking.
        // Non-heap references like reference to stack variables should be skipped.
        let try_mark_push = |mark_queue: &mut VecDeque<ObjectRef>, or: ObjectRef, pin: bool| {
            if !or.is_null() && let Some(segment) = find_segment(or) {
                if segment.get_mut().mark_object(or, pin).unwrap() {
                    mark_queue.push_back(or);
                }
            }
        };
        let mut handle_table_lock = self.handle_table.write().unwrap();

        // ----------
        // Mark phase
        // ----------

        // Scan roots reported by CLR. The majority are on-stack variables,
        // as well as references hold by CLR native code with GC_PROTECT().
        self.clr.scan_roots(generation, 2, true, false, false,
            |pp_obj, _sc, f| {
                let or =
                    if (*pp_obj).is_null() {
                        None
                    } else if f.contains(ScanFlags::MayBeInterior) {
                        // `ref` variables and `ref struct` can point to the non-start position of object
                        // and are called "interior references". We need to find the start of the actual object.
                        find_segment(*pp_obj).and_then(|s| s.find_object(*pp_obj))
                    } else {
                        Some(*pp_obj)
                    };

                if let Some(obj) = or {
                    try_mark_push(&mut mark_queue, obj, f.contains(ScanFlags::Pinned));
                }
            });
        // println!("Encountered {} roots from stack.", mark_queue.len());

        // Strong and pinned handles keep object rooted.
        for h in handle_table_lock.iter().filter(|h| !h.object.is_null()) {
            match h.handle_type {
                HandleType::Strong => try_mark_push(&mut mark_queue, h.object, false),
                HandleType::Pinned => try_mark_push(&mut mark_queue, h.object, true),
                _ => {}
            }
        }
        // println!("Encountered {} roots from stack & handle.", mark_queue.len() - stack_roots);

        // Objects in finalization queue are kept uncleared. Since we do not use extra flag to distinguish
        // whether finalization is completed, just rooting the finalization queue is sufficient.
        for f in self.finalization_queue.lock().unwrap().iter() {
            try_mark_push(&mut mark_queue, *f, false);
        }

        // We have done marking the roots. Mark every object reachable from the roots.
        while !mark_queue.is_empty() {
            while let Some(or) = mark_queue.pop_front() {
                let obj = unsafe { &mut *or };
                obj.for_each_obj_ref(|r| try_mark_push(&mut mark_queue, *r, false));
            }

            // For primary object of dependent handles which are alive, mark its secondary object.
            // This must be done in a loop with regular field propagation, because dependent handle
            // can reach new primary object, either directly or indirectly with fields.
            for h in handle_table_lock.iter().filter(|h| h.handle_type == HandleType::Dependent) {
                if !is_object_dead(h.object) {
                    try_mark_push(&mut mark_queue, h.extra_or_secondary as ObjectRef, false);
                }
            }
        }

        // All the root reachable objects are marked. Clear weak handles now.
        // Weak handles are cleared when the object is eligible for finalization.
        for h in handle_table_lock.iter_mut() {
            if h.handle_type == HandleType::Short && is_object_dead(h.object) {
                h.object = null_mut();
            }
        }

        let has_finalizable;

        // Mark finalizables
        // Objects unreachable from roots are eligible for finalization. Objects reachable from
        // finalizable objects are kept, but they are also eligible for finalization. In other
        // words, a finalizable object can see its field finalized in finalizer.
        {
            let mut finalizables: VecDeque<ObjectRef> = VecDeque::new();
            for seg in r.iter() {
                for or in seg.iter() {
                    let obj = unsafe { &mut *or };
                    if !seg.is_marked(or).unwrap() && obj.needs_finalization() && !seg.get_finalization_pending(or).unwrap() {
                        // If an object is not marked at this stage, it's eligible for finalization.
                        // For object with finalization pending flag set, it will be kept alive by finalization queue,
                        // then by finalizer thread, then unreachable if finalizer completes.
                        finalizables.push_back(or);
                        seg.get_mut().set_finalization_pending(or, true).unwrap();
                        try_mark_push(&mut mark_queue, or, false);
                    }
                }
            }

            // Populate objects reachable from finalizable objects. All such objects will be kept on heap.
            while !mark_queue.is_empty() {
                while let Some(or) = mark_queue.pop_front() {
                    let obj = unsafe { &mut *or };
                    obj.for_each_obj_ref(|r| try_mark_push(&mut mark_queue, *r, false));
                }

                for h in handle_table_lock.iter().filter(|h| h.handle_type == HandleType::Dependent) {
                    if !is_object_dead(h.object) {
                        try_mark_push(&mut mark_queue, h.extra_or_secondary as ObjectRef, false);
                    }
                }
            }

            let mut q = self.finalization_queue.lock().unwrap();
            // println!("Find {} new objects eligible for finalization. Existing in queue: {}", finalizables.len(), q.len());
            q.extend(finalizables);
            has_finalizable = !q.is_empty();
        }

        // All the finalization-reachable objects are marked. Clear recursion-tracking and dependent handles now.
        // Recursion-tracking handles are cleared when the object is unreachable from finalization.
        for h in handle_table_lock.iter_mut().filter(|h| is_object_dead(h.object)) {
            debug_assert!(h.object.is_null() || h.handle_type == HandleType::ShortRecurrsion || h.handle_type == HandleType::Dependent);
            h.object = null_mut();
            if h.handle_type == HandleType::Dependent {
                h.extra_or_secondary = 0;
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

        // ----------
        // Sweep phase
        // ----------
        {
            let mut w = self.segments.write().unwrap();

            // During sweep, the ending of the segment will be modified. This will be break active allocations.
            self.clr.for_each_alloc_context(|c| {
                if c.alloc_limit != 0 {
                    w.iter_mut()
                        .find(|s| s.data().as_ptr_range().end == c.alloc_limit as *const usize)
                        .unwrap()
                        .set_in_use();
                }
            });

            // Do the sweeping. Extract empty segments (sweep returns false).
            let mut empty: VecDeque<_> = w.extract_if(.., |s| !s.get_in_use() && !s.sweep()).collect();

            let heap_count = w.iter().flat_map(|s| s.iter()).count();
            let heap_bytes = w.iter().flat_map(|s| s.iter().map(|or| unsafe { (*or).total_size_aligned() })).sum::<usize>();
            // println!("{} object survived after sweeping. Total size: {}", heap_count, heap_bytes);

            // Segment with usage below threshold are eligible for compating. Pinning an object will make
            // the whole segment not compactible, since leaving a pinned object occupying the whole segment
            // is usually not beneficial.
            const COMPACT_THRESHOLD: usize = Segment::SIZE / 4;
            let dropped_segments: Vec<_> = w
                .extract_if(.., |s|
                    !s.get_in_use() && !s.contains_pinned() && s.alive_bytes() < COMPACT_THRESHOLD)
                .collect();
            if dropped_segments.len() > 1 {
                for seg in dropped_segments.iter() {
                    // Find a destination to move. Prefer existing segments with enough space.
                    // Decide destination by segment and consider trailing space only. This simplifies the algorithm greatfully.
                    let destination = if let Some(s) = w.iter_mut()
                        .find(|s| !s.get_in_use() && s.available_range().len() * size_of::<usize>() > seg.alive_bytes()) {
                            s
                        } else {
                            let new_seg = empty.pop_front().unwrap_or_else(|| UnsafeRef::new(Segment::new_boxed()));
                            w.push(new_seg);
                            w.last_mut().unwrap()
                        };
                    println!("Compacting segment at {:016x} ({} bytes alive) into segment at {:016x} ({} bytes available)", seg.data().as_ptr() as usize, seg.alive_bytes(), destination.data().as_ptr() as usize, destination.available_range().len() * size_of::<usize>());
                    let mut index = destination.available_range().start;
                    for or in seg.iter() {
                        debug_assert!(seg.is_marked(or).unwrap());
                        debug_assert!(!seg.is_pinned(or).unwrap());
                        let ptr_size = unsafe { (*or).total_size_aligned() / size_of::<usize>() };

                        // Copy the object to new destination. Note the offset -1 to include object header,
                        // and exclude next object header.
                        let src = unsafe { &*slice_from_raw_parts((or as *const usize).wrapping_sub(1), ptr_size) };
                        let data = &mut destination.data_mut()[index..];
                        let new_or = &raw const data[1] as ObjectRef;
                        data[..ptr_size].copy_from_slice(src);
                        unsafe {
                            // Store the moved address at object header space. This keeps the old segment traversable.
                            *(or as *mut usize).wrapping_sub(1) = new_or as usize;
                        }
                        // Copy persist flags to new segment.
                        destination.set_finalization_pending(new_or, seg.get_finalization_pending(or).unwrap()).unwrap();
                        // println!("Copied object sized {} from {:016x} to {:016x}", ptr_size * size_of::<usize>(), or as usize, new_or as usize);

                        index += ptr_size;
                    }
                    destination.update_available_range(index);
                    println!("Dropping moved segment at {:016x}", seg.data().as_ptr() as usize);
                }

                for e in empty.into_iter() {
                    println!("Dropping empty segment at {:016x}", e.data().as_ptr() as usize);
                }

                // Modify all references to moved object. Extract the new address from object header space.
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

                // Traverse roots reported by CLR. Set promotion=false to the call for updating.
                self.clr.scan_roots(generation, 2, false, false, false,
                    |pp_obj, _, f| {
                        if f.contains(ScanFlags::MayBeInterior) {
                            fix_ref_interior(pp_obj);
                        } else {
                            fix_ref(pp_obj);
                        }
                    });
                // Traverse the entire heap to find reference in fields. This is the longest STW step in concurrent GC,
                // and generational GC optimizes this by explicitly tracking cross-generation references.
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
                for f in self.finalization_queue.lock().unwrap().iter_mut() {
                    fix_ref(f);
                }
            } else {
                // Avoid allocating a new segment for 1 compactible segment. Since it's usually not beneficial,
                // just cancel the compaction.
                w.extend(dropped_segments);
            }

            // Clear all transient flags like mark and pin. Permanent flags like finalization are preserved.
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

    /// Retrieves a finalizable object by the finalization thread.
    /// Once extracted, the object will be rooted by the finalization thread until finalization is done.
    /// The thread will not reach safe point before it stores the reference.
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
