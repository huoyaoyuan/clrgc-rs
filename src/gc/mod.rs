use std::collections::VecDeque;
use std::sync::RwLock;
use std::vec;

pub use handle_manager::HandleManager;
pub use segment::{Seg, Segment, LargeSegment};
use unsafe_ref::UnsafeRef;
use crate::gcinterface::{GCToCLR, IGCToCLR, ScanFlags, SuspendReason};
use crate::objects::ObjectRef;

mod handle_manager;
mod segment;
mod unsafe_ref;

pub struct RustGc {
    pub clr: GCToCLR,
    pub handle_manager: HandleManager,
    segments: RwLock<Vec<UnsafeRef<dyn Seg>>>,
}

impl RustGc {
    pub fn new(clr: *const IGCToCLR) -> RustGc {
        RustGc {
            clr: GCToCLR::new(clr),
            handle_manager: HandleManager::new(),
            segments: RwLock::new(vec![]),
        }
    }

    pub fn add_segment(&mut self, size: usize) -> UnsafeRef<dyn Seg> {
        let new_seg : Box<dyn Seg> =
            if size <= Segment::SIZE {
                Box::new(Segment::new())
            } else {
                Box::new(LargeSegment::new(size))
            };
        let r = UnsafeRef::new(new_seg);
        let mut w = self.segments.write().unwrap();
        w.push(r.clone());
        r
    }

    pub fn try_find_interior(&self, or_maybe: ObjectRef) -> Option<ObjectRef> {
        let r = self.segments.read().unwrap();
        let segment = r.iter().find(|s| { s.contains(or_maybe) })?;
        segment.find_object(or_maybe)
    }

    fn mark_object(&self, or: ObjectRef) -> Result<bool, ()> {
        let r = self.segments.write().unwrap();
        let segment = r.iter().find(|s| { s.contains(or) } ).ok_or(())?;
        segment.get_mut().mark_object(or)
    }

    pub fn do_collect(&mut self, generation: i32) {
        println!("GC triggered for generation {}", generation);
        
        println!("Suspending EE");
        self.clr.suspend_ee(SuspendReason::GC);

        self.clr.gc_start_work(generation, 2);

        let mut mark_queue : VecDeque<ObjectRef> = VecDeque::new();
        self.clr.scan_roots(generation, 2, true, false, false,
            |pp_obj, _sc, f| {
                let or =
                    if (*pp_obj).is_null() {
                        None
                    } else if f.contains(ScanFlags::MayBeInterior) {
                        self.try_find_interior(*pp_obj)
                    } else {
                        Some(*pp_obj)
                    };

                if let Some(obj) = or && self.mark_object(obj).unwrap_or(false) {
                    mark_queue.push_back(obj);
                }
            });
        println!("Encountered totally {} on-heap roots during scan.", mark_queue.len());

        let mut heap_count : i32 = 0;
        let mut heap_bytes : u32 = 0;
        let mut marked_count : i32 = 0;
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
                    }
                });
            }
        }
        println!("Encountered totally {} objects on heap. Total size: {} bytes. Marked: {}.", heap_count, heap_bytes, marked_count);

        {
            let mut w = self.segments.write().unwrap();
            for seg in w.iter_mut() {
                seg.clear_mark();
            }
        }
        
        println!("Resuming EE");
        self.clr.gc_done(generation);
        self.clr.restart_ee(true);
    }
}
