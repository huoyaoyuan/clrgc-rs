use std::sync::RwLock;
use crate::{ObjectRef, HandleType};

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct GcHandle {
    pub object: ObjectRef,
    pub extra_or_secondary: usize,
    pub handle_type: HandleType,
}

pub type ObjectHandle = *mut GcHandle;

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
        r.handles[0..(r.used_handles)].as_ptr_range().contains(&(handle as *const GcHandle))
    }

    pub fn create_handle(&mut self, object: ObjectRef, extra_or_secondary: usize, handle_type: HandleType) -> ObjectHandle {
        let mut w = self.handle_table.write().unwrap();
        let idx = w.used_handles;
        w.handles[idx] = GcHandle { object, extra_or_secondary, handle_type };
        w.used_handles = idx + 1;
        &mut w.handles[idx]
    }

    pub fn duplicate_handle(&mut self, handle: ObjectHandle) -> ObjectHandle {
        let mut w = self.handle_table.write().unwrap();
        let h = unsafe { *(handle as *const GcHandle) };
        let idx = w.used_handles;
        w.handles[idx] = h;
        w.used_handles = idx + 1;
        &mut w.handles[idx]
    }

    pub fn destroy_handle(&mut self, handle: ObjectHandle) {
        let _w = self.handle_table.write().unwrap();
        let h = handle as *mut GcHandle;
        unsafe { *h = GcHandle::default() };
    }
}
