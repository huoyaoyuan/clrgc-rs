use std::{sync::{Arc, RwLock}, vec};

pub use handle_manager::{ObjectHandle, HandleManager};
pub use segment::Segment;
use crate::gcinterface::{GCToCLR, IGCToCLR};

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
}
