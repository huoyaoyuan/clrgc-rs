use crate::{ObjectHandle, ObjectRef, HandleType};
use crate::gc::RustGc;

#[repr(C)]
pub struct IGCHandleStore {
    vptr: *const IGCHandleStoreVTable,
    gc: *mut RustGc,
}

#[repr(C)]
pub struct IGCHandleStoreVTable {
    Uproot: extern "system" fn (this: *mut IGCHandleStore),
    ContainsHandle: extern "system" fn (this: *mut IGCHandleStore, handle: ObjectHandle) -> bool,
    CreateHandleOfType: extern "system" fn (this: *mut IGCHandleStore, obj: ObjectRef, t: HandleType) -> ObjectHandle,
    CreateHandleOfType2: extern "system" fn (this: *mut IGCHandleStore, obj: ObjectRef, t: HandleType, heapToAffinitizeTo: i32) -> ObjectHandle,
    CreateHandleWithExtraInfo: extern "system" fn (this: *mut IGCHandleStore, obj: ObjectRef, t: HandleType, extra_info: usize) -> ObjectHandle,
    CreateDependentHandle: extern "system" fn (this: *mut IGCHandleStore, primary: ObjectRef, secondary: ObjectRef) -> ObjectHandle,
    Destruct: extern "system" fn (this: *mut IGCHandleStore),
}

fn get_gc_store(this: *mut IGCHandleStore) -> &'static mut RustGc {
    unsafe { &mut *(*this).gc }
}

extern "system" fn GCHandleStore_Nop(_: *mut IGCHandleStore) {
}

extern "system" fn GCHandleStore_ContainsHandle(this: *mut IGCHandleStore, handle: ObjectHandle) -> bool {
    get_gc_store(this).handle_manager.contains_handle(handle)
}

extern "system" fn GCHandleStore_CreateHandleOfType(this: *mut IGCHandleStore, obj: ObjectRef, t: HandleType) -> ObjectHandle {
    get_gc_store(this).handle_manager.create_handle(obj, 0, t)
}

extern "system" fn GCHandleStore_CreateHandleOfType2(this: *mut IGCHandleStore, obj: ObjectRef, t: HandleType, _: i32) -> ObjectHandle {
    get_gc_store(this).handle_manager.create_handle(obj, 0, t)
}

extern "system" fn GCHandleStore_CreateHandleWithExtraInfo(this: *mut IGCHandleStore, obj: ObjectRef, t: HandleType, extra_info: usize) -> ObjectHandle {
    get_gc_store(this).handle_manager.create_handle(obj, extra_info, t)
}

extern "system" fn GCHandleStore_CreateDependentHandle(this: *mut IGCHandleStore, primary: ObjectRef, secondary: ObjectRef) -> ObjectHandle {
    get_gc_store(this).handle_manager.create_handle(primary, secondary as usize, HandleType::Dependent)
}

const GCHandleStore_vtable : IGCHandleStoreVTable = IGCHandleStoreVTable {
    Uproot: GCHandleStore_Nop,
    ContainsHandle: GCHandleStore_ContainsHandle,
    CreateHandleOfType: GCHandleStore_CreateHandleOfType,
    CreateHandleOfType2: GCHandleStore_CreateHandleOfType2,
    CreateHandleWithExtraInfo: GCHandleStore_CreateHandleWithExtraInfo,
    CreateDependentHandle: GCHandleStore_CreateDependentHandle,
    Destruct: GCHandleStore_Nop,
};

#[repr(C)]
pub struct IGCHandleManager {
    vptr: *const IGCHandleManagerVTable,
    handle_store: IGCHandleStore,
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
        IGCHandleManager {
            vptr: &GCHandleManager_vtable,
            handle_store: IGCHandleStore { vptr: &GCHandleStore_vtable, gc },
            gc,
        }
    }
}
