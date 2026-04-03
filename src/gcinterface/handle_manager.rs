use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};
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
    Initialize: extern "system" fn (this: *mut IGCHandleManager) -> bool,
    Shutdown: extern "system" fn (this: *mut IGCHandleManager),
    GetGlobalHandleStore: extern "system" fn (this: *mut IGCHandleManager) -> *const IGCHandleStore,
    CreateHandleStore: extern "system" fn (this: *mut IGCHandleManager) -> *const IGCHandleStore,
    DestroyHandleStore: extern "system" fn (this: *mut IGCHandleManager, *mut IGCHandleStore),
    CreateGlobalHandleOfType: extern "system" fn (this: *mut IGCHandleManager, obj: ObjectRef, t: HandleType) -> ObjectHandle,
    CreateDuplicateHandle: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle) -> ObjectHandle,
    DestroyHandleOfType: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle, t: HandleType),
    DestroyHandleOfUnknownType: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle),
    SetExtraInfoForHandle: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle, extra_info: usize),
    GetExtraInfoFromHandle: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle) -> usize,
    StoreObjectInHandle: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle, object: ObjectRef),
    StoreObjectInHandleIfNull: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle, object: ObjectRef),
    SetDependentHandleSecondary: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle, object: usize),
    GetDependentHandleSecondary: extern "system" fn (this: *mut IGCHandleManager, object: ObjectHandle) -> usize,
    InterlockedCompareExchangeObjectInHandle: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle, object: ObjectRef, comparand: ObjectRef) -> ObjectRef,
    HandleFetchType: extern "system" fn (this: *mut IGCHandleManager, handle: ObjectHandle) -> HandleType,
    TraceRefCountedHandles: usize,
}

fn get_gc(this: *mut IGCHandleManager) -> &'static mut RustGc {
    unsafe {
        &mut *(*this).gc
    }
}

extern "system" fn GCHandleManager_Initialize(_: *mut IGCHandleManager) -> bool {
    true
}

extern "system" fn GCHandleManager_Nop(_: *mut IGCHandleManager) {
}

extern "system" fn GCHandleManager_GetGlobalHandleStore(this: *mut IGCHandleManager) -> *const IGCHandleStore {
    unsafe { &(*this).handle_store }
}

extern "system" fn GCHandleManager_CreateHandleStore(_: *mut IGCHandleManager) -> *const IGCHandleStore {
    unimplemented!()
}

extern "system" fn GCHandleManager_DestroyHandleStore(_: *mut IGCHandleManager, _: *mut IGCHandleStore) {
    unimplemented!()
}

extern "system" fn GCHandleManager_CreateGlobalHandleOfType(this: *mut IGCHandleManager, obj: ObjectRef, t: HandleType) -> ObjectHandle {
    get_gc(this).handle_manager.create_handle(obj, 0, t)
}

extern "system" fn GCHandleManager_CreateDuplicateHandle(this: *mut IGCHandleManager, handle: ObjectHandle) -> ObjectHandle {
    get_gc(this).handle_manager.duplicate_handle(handle)
}

extern "system" fn GCHandleManager_DestroyHandleOfType(this: *mut IGCHandleManager, handle: ObjectHandle, _: HandleType) {
    get_gc(this).handle_manager.destroy_handle(handle)
}

extern "system" fn GCHandleManager_DestroyHandleOfUnknownType(this: *mut IGCHandleManager, handle: ObjectHandle) {
    get_gc(this).handle_manager.destroy_handle(handle)
}

extern "system" fn GCHandleManager_SetExtraOrSecondary(_: *mut IGCHandleManager, handle: ObjectHandle, extra_info: usize) {
    unsafe { (*handle).extra_or_secondary = extra_info; }
}

extern "system" fn GCHandleManager_GetExtraOrSecondary(_: *mut IGCHandleManager, handle: ObjectHandle) -> usize {
    unsafe { (*handle).extra_or_secondary }
}

extern "system" fn GCHandleManager_StoreObjectInHandle(_: *mut IGCHandleManager, handle: ObjectHandle, obj: ObjectRef) {
    unsafe { (*handle).object = obj; }
}

extern "system" fn GCHandleManager_StoreObjectInHandleIfNull(_: *mut IGCHandleManager, handle: ObjectHandle, obj: ObjectRef) {
    let a = unsafe { AtomicPtr::from_ptr(&mut (*handle).object) };
    _ = a.compare_exchange(null_mut(), obj, Ordering::AcqRel, Ordering::Relaxed);
}

extern "system" fn GCHandleManager_InterlockedCompareExchangeObjectInHandle(_: *mut IGCHandleManager, handle: ObjectHandle, obj: ObjectRef, comparand: ObjectRef) -> ObjectRef {
    let a = unsafe { AtomicPtr::from_ptr(&mut (*handle).object) };
    let r = a.compare_exchange(comparand, obj, Ordering::AcqRel, Ordering::Relaxed);
    match r {
        Ok(v) => v,
        Err(v) => v,
    }
}

extern "system" fn GCHandleManager_HandleFetchType(_: *mut IGCHandleManager, handle: ObjectHandle) -> HandleType {
    unsafe { (*handle).handle_type }
}

const GCHandleManager_vtable : IGCHandleManagerVTable = IGCHandleManagerVTable {
    Initialize: GCHandleManager_Initialize,
    Shutdown: GCHandleManager_Nop,
    GetGlobalHandleStore: GCHandleManager_GetGlobalHandleStore,
    CreateHandleStore: GCHandleManager_CreateHandleStore,
    DestroyHandleStore: GCHandleManager_DestroyHandleStore,
    CreateGlobalHandleOfType: GCHandleManager_CreateGlobalHandleOfType,
    CreateDuplicateHandle: GCHandleManager_CreateDuplicateHandle,
    DestroyHandleOfType: GCHandleManager_DestroyHandleOfType,
    DestroyHandleOfUnknownType: GCHandleManager_DestroyHandleOfUnknownType,
    SetExtraInfoForHandle: GCHandleManager_SetExtraOrSecondary,
    GetExtraInfoFromHandle: GCHandleManager_GetExtraOrSecondary,
    StoreObjectInHandle: GCHandleManager_StoreObjectInHandle,
    StoreObjectInHandleIfNull: GCHandleManager_StoreObjectInHandleIfNull,
    SetDependentHandleSecondary: GCHandleManager_SetExtraOrSecondary,
    GetDependentHandleSecondary: GCHandleManager_GetExtraOrSecondary,
    InterlockedCompareExchangeObjectInHandle: GCHandleManager_InterlockedCompareExchangeObjectInHandle,
    HandleFetchType: GCHandleManager_HandleFetchType,
    TraceRefCountedHandles: 0,
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
