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
    Uproot: unsafe extern "system" fn (this: *mut IGCHandleStore),
    ContainsHandle: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle) -> bool,
    CreateHandleOfType: unsafe extern "system" fn (this: *mut IGCHandleStore, obj: *const Object, t: handle_type) -> object_handle,
    CreateHandleOfType2: unsafe extern "system" fn (this: *mut IGCHandleStore, obj: *const Object, t: handle_type, heapToAffinitizeTo: i32) -> object_handle,
    CreateHandleWithExtraInfo: unsafe extern "system" fn (this: *mut IGCHandleStore, obj: *const Object, extra_info: isize) -> bool,
    CreateDependentHandle: unsafe extern "system" fn (this: *mut IGCHandleStore, primary: *const Object, secondary: *const Object) -> bool,
    Destruct: unsafe extern "system" fn (this: *mut IGCHandleStore),
}

#[repr(C)]
pub struct IGCHandleManager {
    vptr: *const IGCHandleManagerVTable,
}

#[repr(C)]
pub struct IGCHandleManagerVTable {
    Initialize: unsafe extern "system" fn (this: *mut IGCHandleManager),
    Shutdown: unsafe extern "system" fn (this: *mut IGCHandleManager),
    GetGlobalHandleStore: unsafe extern "system" fn (this: *mut IGCHandleManager) -> *const IGCHandleStore,
    CreateHandleStore: unsafe extern "system" fn (this: *mut IGCHandleManager) -> *const IGCHandleStore,
    DestroyHandleStore: unsafe extern "system" fn (this: *mut IGCHandleManager, *mut IGCHandleStore),
    CreateGlobalHandleOfType: unsafe extern "system" fn (this: *mut IGCHandleStore, obj: *const Object, t: handle_type) -> object_handle,
    CreateDuplicateHandle: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle) -> object_handle,
    DestroyHandleOfType: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, t: handle_type),
    DestroyHandleOfUnknownType: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle),
    SetExtraInfoForHandle: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, extra_info: isize),
    GetExtraInfoFromHandle: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle) -> isize,
    StoreObjectInHandle: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, object: *const Object),
    StoreObjectInHandleIfNull: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, object: *const Object),
    SetDependentHandleSecondary: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle, object: *const Object),
    GetDependentHandleSecondary: unsafe extern "system" fn (this: *mut IGCHandleStore, object: *const Object) -> *const Object,
    InterlockedCompareExchangeObjectInHandle: unsafe extern "system" fn (this: *mut IGCHandleStore, object: *const Object, comparand: *const Object) -> *const Object,
    HandleFetchType: unsafe extern "system" fn (this: *mut IGCHandleStore, handle: object_handle) -> handle_type,
    TraceRefCountedHandles: unsafe extern "system" fn (this: *mut IGCHandleStore, callback: isize, param1: usize, param2: usize) -> handle_type,
}
