pub use handle_manager::{ObjectHandle, HandleManager};
use crate::gcinterface::{GCToCLR, IGCToCLR};

mod handle_manager;

pub struct RustGc {
    pub clr: GCToCLR,
    pub handle_manager: HandleManager,
}

impl RustGc {
    pub fn new(clr: *const IGCToCLR) -> RustGc {
        RustGc {
            clr: GCToCLR::new(clr),
            handle_manager: HandleManager::new(),
        }
    }
}
