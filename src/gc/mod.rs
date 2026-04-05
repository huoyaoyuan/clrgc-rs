use std::{sync::{Arc, RwLock}, vec};

pub use handle_manager::HandleManager;
pub use segment::Segment;
use crate::gcinterface::{GCToCLR, IGCToCLR, ScanFlags, SuspendReason};
use crate::objects::ObjectRef;

mod handle_manager;
mod segment;

pub struct RustGc {
    pub clr: GCToCLR,
    pub handle_manager: HandleManager,
    segments: RwLock<Vec<Arc<Segment>>>,
}

impl RustGc {
    pub fn new(clr: *const IGCToCLR) -> RustGc {
        RustGc {
            clr: GCToCLR::new(clr),
            handle_manager: HandleManager::new(),
            segments: RwLock::new(vec![]),
        }
    }

    pub fn add_segment(&mut self, size: usize) -> Arc<Segment> {
        let mut w = self.segments.write().unwrap();
        w.push(Arc::new(Segment::new(size)));
        let arc = w.last().unwrap();
        arc.clone()
    }

    pub fn try_find_interior(&self, or_maybe: ObjectRef) -> Option<ObjectRef> {
        let r = self.segments.read().unwrap();
        let segment = r.iter().find(|s| { s.contains(or_maybe) })?;
        segment.find_object(or_maybe)
    }

    pub fn do_collect(&mut self, generation: i32) {
        println!("GC triggered for generation {}", generation);
        
        println!("Suspending EE");
        self.clr.suspend_ee(SuspendReason::GC);

        self.clr.gc_start_work(generation, 2);

        let mut c : i32 = 0;
        self.clr.scan_roots(generation, 2, true, false, false,
            |or, _sc, f| {
                c += 1;
                unsafe {
                    print!("Root at {:016x}, object: {:016x}, ", or as *const ObjectRef as usize, *or as usize);
                    if (*or).is_null() {
                        println!("null");
                    } else if f.contains(ScanFlags::MayBeInterior) {
                        match self.try_find_interior(*or) {
                            None => println!("interior: not on heap"),
                            Some(obj) => println!("interior of {:016x}, Total Size: {}", obj as usize, (*obj).total_size()),
                        };
                    }
                     else {
                        let mt = (**or).method_table;
                        println!("Has ComponentSize: {}, ComponentSize: {}, ComponentCount: {}, Total Size: {}", (**or).has_component_size(), (*mt).component_size, (**or).component_count, (**or).total_size());
                    }
                }
            });
        println!("Encountered totally {} roots during scan.", c);

        let mut heap_count : i32 = 0;
        let mut heap_bytes : u32 = 0;
        {
            let r = self.segments.read().unwrap();
            for seg in r.iter() {
                seg.for_each_obj(|or| {
                    unsafe {
                        // println!("Walking at {:016x}, MethodTable: {:016x}", or as usize, (*or).method_table as usize);
                        // println!("Object: HasComponentSize: {}, TotalSize: {}", (*or).has_component_size(), (*or).total_size());
                        heap_count += 1;
                        heap_bytes += (*or).total_size();
                    }
                });
            }
        }
        println!("Encountered totally {} objects on heap. Total size: {} bytes.", heap_count, heap_bytes);
        
        println!("Resuming EE");
        self.clr.gc_done(generation);
        self.clr.restart_ee(true);
    }
}
