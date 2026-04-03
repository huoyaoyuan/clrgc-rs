pub use handle_manager::HandleManager;

mod handle_manager;

pub struct RustGc {
    handle_manager: HandleManager,
}

impl RustGc {
    pub fn new() -> RustGc {
        RustGc { handle_manager: HandleManager::new() }
    }
}
