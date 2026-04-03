use crate::gc::RustGc;
use super::Object;

type object_handle = usize;

#[repr(i32)]
pub enum handle_type {
    HNDTYPE_WEAK_SHORT = 0,
    HNDTYPE_WEAK_DEFAULT = 1,
    HNDTYPE_STRONG = 2,
    HNDTYPE_PINNED = 3,
}

#[repr(C)]
pub struct IGCHandleStore {
    vptr: *const IGCHandleStoreVTable,
}

#[repr(C)]
pub struct IGCHandleStoreVTable {
    Uproot: extern "system" fn (this: *mut IGCHandleStore),
    ContainsHandle: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle) -> bool,
    CreateHandleOfType: extern "system" fn (this: *mut IGCHandleStore, obj: *const Object, t: handle_type) -> object_handle,
    CreateHandleOfType2: extern "system" fn (this: *mut IGCHandleStore, obj: *const Object, t: handle_type, heapToAffinitizeTo: i32) -> object_handle,
    CreateHandleWithExtraInfo: extern "system" fn (this: *mut IGCHandleStore, obj: *const Object, extra_info: isize) -> bool,
    CreateDependentHandle: extern "system" fn (this: *mut IGCHandleStore, primary: *const Object, secondary: *const Object) -> bool,
    Destruct: extern "system" fn (this: *mut IGCHandleStore),
}

#[repr(C)]
pub struct IGCHandleManager {
    vptr: *const IGCHandleManagerVTable,
    gc: *mut RustGc,
}

#[repr(C)]
pub struct IGCHandleManagerVTable {
    Initialize: extern "system" fn (this: *mut IGCHandleManager),
    Shutdown: extern "system" fn (this: *mut IGCHandleManager),
    /* GetGlobalHandleStore: extern "system" fn (this: *mut IGCHandleManager) -> *const IGCHandleStore,
    CreateHandleStore: extern "system" fn (this: *mut IGCHandleManager) -> *const IGCHandleStore,
    DestroyHandleStore: extern "system" fn (this: *mut IGCHandleManager, *mut IGCHandleStore),
    CreateGlobalHandleOfType: extern "system" fn (this: *mut IGCHandleStore, obj: *const Object, t: handle_type) -> object_handle,
    CreateDuplicateHandle: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle) -> object_handle,
    DestroyHandleOfType: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, t: handle_type),
    DestroyHandleOfUnknownType: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle),
    SetExtraInfoForHandle: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, extra_info: isize),
    GetExtraInfoFromHandle: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle) -> isize,
    StoreObjectInHandle: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, object: *const Object),
    StoreObjectInHandleIfNull: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, object: *const Object),
    SetDependentHandleSecondary: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, object: *const Object),
    GetDependentHandleSecondary: extern "system" fn (this: *mut IGCHandleStore, object: *const Object) -> *const Object,
    InterlockedCompareExchangeObjectInHandle: extern "system" fn (this: *mut IGCHandleStore, object: *const Object, comparand: *const Object) -> *const Object,
    HandleFetchType: extern "system" fn (this: *mut IGCHandleStore, handle: object_handle) -> handle_type,
    TraceRefCountedHandles: extern "system" fn (this: *mut IGCHandleStore, callback: isize, param1: usize, param2: usize) -> handle_type, */
}

fn get_gc(this: *mut IGCHandleManager) -> &'static mut RustGc {
    unsafe {
        &mut *(*this).gc
    }
}

extern "system" fn GCHandleManager_Nop(_: *mut IGCHandleManager) {
}

const GCHandleManager_vtable : IGCHandleManagerVTable = IGCHandleManagerVTable {
    Initialize: GCHandleManager_Nop,
    Shutdown: GCHandleManager_Nop,
    /* GetGlobalHandleStore: todo!(),
    CreateHandleStore: todo!(),
    DestroyHandleStore: todo!(),
    CreateGlobalHandleOfType: todo!(),
    CreateDuplicateHandle: todo!(),
    DestroyHandleOfType: todo!(),
    DestroyHandleOfUnknownType: todo!(),
    SetExtraInfoForHandle: todo!(),
    GetExtraInfoFromHandle: todo!(),
    StoreObjectInHandle: todo!(),
    StoreObjectInHandleIfNull: todo!(),
    SetDependentHandleSecondary: todo!(),
    GetDependentHandleSecondary: todo!(),
    InterlockedCompareExchangeObjectInHandle: todo!(),
    HandleFetchType: todo!(),
    TraceRefCountedHandles: todo!(), */
};

impl IGCHandleManager {
    pub fn new(gc: *mut RustGc) -> IGCHandleManager {
        IGCHandleManager { vptr: &GCHandleManager_vtable, gc }
    }
}
