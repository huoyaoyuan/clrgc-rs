use std::ffi::{c_char, c_void};
use super::gc_heap::gc_alloc_context;
use crate::Object;

#[repr(C)]
pub struct IGCToCLR {
    vptr: *const IGCToClrVTable,
}

#[repr(C)]
pub struct ScanContext {
    thread_under_crawl: isize,
    thread_number: i32,
    thread_count: i32,
    stack_limit: usize,
    promotion: bool,
    concurrent: bool,
    _unused1: isize,
    pMD: isize,
    _unused3: i32,
}

type promote_func = unsafe extern "system" fn(ppObject: *const *mut Object, sc: *const ScanContext, flags: u32);

#[repr(i32)]
pub enum SUSPEND_REASON {
    SUSPEND_FOR_GC = 1,
    SUSPEND_FOR_GC_PREP = 6,
}

#[repr(C)]
struct IGCToClrVTable {
    SuspendEE: extern "system" fn(this: *const IGCToCLR, reason: SUSPEND_REASON),
    RestartEE: extern "system" fn(this: *const IGCToCLR, bFinishedGC: bool),
    GcScanRoots: extern "system" fn(this: *const IGCToCLR, func: promote_func, condemned: i32, max_gen: i32, sc: *const ScanContext),
    GcStartWork: extern "system" fn(this: *const IGCToCLR, condemned: i32, max_gen: i32),
    BeforeGcScanRoots: extern "system" fn(this: *const IGCToCLR, condemned: i32, is_bgc: bool, is_concurrent: bool),
    AfterGcScanRoots: extern "system" fn(this: *const IGCToCLR, condemned: i32, is_bgc: bool, sc: *const ScanContext),
    GcDone: extern "system" fn(this: *const IGCToCLR, condemned: i32),
    RefCountedHandleCallbacks: isize,
    SyncBlockCacheWeakPtrScan: isize,
    SyncBlockCacheDemote: isize,
    SyncBlockCachePromotionsGranted: isize,
    GetActiveSyncBlockCount: isize,
    IsPreemptiveGCDisabled: extern "system" fn(this: *const IGCToCLR) -> bool,
    EnablePreemptiveGC: extern "system" fn(this: *const IGCToCLR) -> bool,
    DisablePreemptiveGC: extern "system" fn(this: *const IGCToCLR) -> bool,
    GetThread: extern "system" fn(this: *const IGCToCLR) -> isize,
    GetAllocContext: extern "system" fn(this: *const IGCToCLR) -> *const gc_alloc_context,
    GcEnumAllocContexts: extern "system" fn(this: *const IGCToCLR, func: extern "system" fn(*const gc_alloc_context, *const c_void), param: *const c_void),
    GetLoaderAllocatorObjectForGC: extern "system" fn(this: *const IGCToCLR, object: *const Object) -> isize,
    CreateThread: extern "system" fn(this: *const IGCToCLR, thread_start: extern "system" fn(*const c_void), arg: *const c_void, is_suspendable: bool, name: *const c_char),
}

pub struct GCToCLR {
    ptr: *const IGCToCLR,
}

impl GCToCLR {
    fn vtable(&self) -> &'static IGCToClrVTable {
        unsafe {
            let obj: &IGCToCLR = &*self.ptr;
            &*obj.vptr
        }
    }

    pub fn suspend_ee(&self, reason: SUSPEND_REASON) {
        (self.vtable().SuspendEE)(self.ptr, reason)
    }

    pub fn restart_ee(&self, finished_gc: bool) {
        (self.vtable().RestartEE)(self.ptr, finished_gc)
    }

    pub fn gc_scan_roots(&self, func: promote_func, generation: i32, max_gen: i32, scan_context: *const ScanContext) {
        (self.vtable().GcScanRoots)(self.ptr, func, generation, max_gen, scan_context)
    }

    pub fn gc_start_work(&self, generation: i32, max_gen: i32) {
        (self.vtable().GcStartWork)(self.ptr, generation, max_gen)
    }

    pub fn before_gc_scan_roots(&self, generation: i32, is_bgc: bool, is_concurrent: bool) {
        (self.vtable().BeforeGcScanRoots)(self.ptr, generation, is_bgc, is_concurrent)
    }

    pub fn after_gc_scan_roots(&self, generation: i32, is_bgc: bool, scan_context: *const ScanContext) {
        (self.vtable().AfterGcScanRoots)(self.ptr, generation, is_bgc, scan_context)
    }

    pub fn gc_done(&self, generation: i32) {
        (self.vtable().GcDone)(self.ptr, generation)
    }

    pub fn get_allocate_context(&self) -> &'static gc_alloc_context {
        unsafe { &*(self.vtable().GetAllocContext)(self.ptr) }
    }

    pub fn for_each_alloc_context(&self, action: fn(&gc_alloc_context)) {
        let closure = Box::new(action);
        extern "system" fn callback(alloc_context: *const gc_alloc_context, param: *const c_void) {
            unsafe {
                let action = &*(param as *mut fn(&gc_alloc_context));
                action(&*alloc_context);
            }
        }
        (self.vtable().GcEnumAllocContexts)(self.ptr, callback, Box::into_raw(closure) as *const c_void)
    }
}
