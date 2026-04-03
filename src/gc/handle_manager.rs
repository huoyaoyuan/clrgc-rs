use std::sync::RwLock;
use crate::{ObjectHandle, ObjectRef, HandleType};

#[repr(C)]
#[derive(Default, Clone)]
struct GcHandle {
    object: ObjectRef,
    extra_or_secondary: usize,
    handle_type: HandleType,
}

pub struct HandleManager {
    handle_table: RwLock<HandleTable>,
}

struct HandleTable {
    handles: Box<[GcHandle]>,
    used_handles: usize,
}

impl HandleManager {
    pub fn new() -> HandleManager {
        let handle_table = HandleTable {
            handles: vec![GcHandle::default(); 65536].into_boxed_slice(),
            used_handles: 0,
        };
        HandleManager { handle_table: RwLock::new(handle_table) }
    }

    pub fn contains_handle(&self, handle: ObjectHandle) -> bool {
        let r = self.handle_table.read().unwrap();
        r.handles.as_ptr_range().contains(&(handle as *const GcHandle))
    }

    pub fn create_handle(&mut self, object: ObjectRef, extra_or_secondary: usize, handle_type: HandleType) -> ObjectHandle {
        let mut w = self.handle_table.write().unwrap();
        let idx = w.used_handles;
        w.handles[idx] = GcHandle { object, extra_or_secondary, handle_type };
        w.used_handles = idx + 1;
        &w.handles[idx] as *const GcHandle as usize
    }
}
