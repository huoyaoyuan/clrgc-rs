use std::{sync::{Arc, RwLock}, vec};

pub use handle_manager::{ObjectHandle, HandleManager};
pub use segment::Segment;
use crate::gcinterface::{GCToCLR, IGCToCLR, SuspendReason};

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

    pub fn do_collect(&mut self, generation: i32) {
        println!("GC triggered for generation {}", generation);
        
        println!("Suspending EE");
        self.clr.suspend_ee(SuspendReason::GC);

        self.clr.gc_start_work(generation, 2);

        let mut c : i32 = 0;
        self.clr.scan_roots(generation, 2, true, false, false,
            |_or, _sc, _f| {
                c += 1;
            });
        println!("Encountered totally {} roots during scan.", c);
        
        println!("Resuming EE");
        self.clr.gc_done(generation);
        self.clr.restart_ee(true);
    }
}
