use std::ffi::{c_char, c_void};
use bitflags::bitflags;

use super::gc_heap::gc_alloc_context;
use crate::objects::ObjectRef;

#[repr(C)]
pub struct IGCToCLR {
    vptr: *const IGCToClrVTable,
}

#[repr(C)]
#[derive(Default)]
pub struct ScanContext {
    thread_under_crawl: isize,
    thread_number: i32,
    thread_count: i32,
    stack_limit: usize,
    promotion: bool,
    concurrent: bool,
    _unused1: usize,
    pMD: usize,
    _unused3: i32,
}

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct ScanFlags : i32 {
        const MayBeInterior = 1;
        const Pinned = 2;
    }
}

#[repr(i32)]
pub enum SuspendReason {
    GC = 1,
    GCPrep = 6,
}

#[repr(i32)]
#[derive(Default)]
pub enum WriteBarrierOp {
    #[default]
    StompResize = 0,
    StompEphemeral = 1,
    Initialize = 2,
    SwitchToWriteWatch = 3,
    SwitchToNonWriteWatch = 4,
}

#[repr(C)]
#[derive(Default)]
pub struct WriteBarrierParameters {
    pub operation: WriteBarrierOp,
    pub is_runtime_suspended: bool,
    pub requires_upper_bounds_check: bool,
    pub card_table: usize,
    pub card_bundle_table: usize,
    pub lowest_address: usize,
    pub highest_address: usize,
    pub ephemeral_low: usize,
    pub ephemeral_high: usize,
    pub write_watch_table: usize,
    pub region_to_generation_table: usize,
    pub region_shr: usize,
    pub region_use_bitwise_write_barrier: bool,
}

#[repr(C)]
struct IGCToClrVTable {
    SuspendEE: extern "system" fn(this: *const IGCToCLR, reason: SuspendReason),
    RestartEE: extern "system" fn(this: *const IGCToCLR, bFinishedGC: bool),
    GcScanRoots: extern "system" fn(this: *const IGCToCLR, func: extern "system" fn(ppObject: *mut ObjectRef, sc: *const ScanContext, flags: ScanFlags), condemned: i32, max_gen: i32, sc: *const ScanContext),
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
    GcEnumAllocContexts: extern "system" fn(this: *const IGCToCLR, func: extern "system" fn(*const gc_alloc_context, usize), param: usize),
    GetLoaderAllocatorObjectForGC: extern "system" fn(this: *const IGCToCLR, object: ObjectRef) -> isize,
    CreateThread: extern "system" fn(this: *const IGCToCLR, thread_start: extern "system" fn(*const c_void), arg: *const c_void, is_suspendable: bool, name: *const c_char),
    diag: [usize; 7],
    StompWriteBarrier: extern "system" fn(this: *const IGCToCLR, args: *const WriteBarrierParameters),
}

pub struct GCToCLR {
    ptr: *const IGCToCLR,
}

impl GCToCLR {
    pub fn new(ptr: *const IGCToCLR) -> Self {
        Self { ptr }
    }

    fn vtable(&self) -> &'static IGCToClrVTable {
        unsafe {
            let obj: &IGCToCLR = &*self.ptr;
            &*obj.vptr
        }
    }

    pub fn suspend_ee(&self, reason: SuspendReason) {
        (self.vtable().SuspendEE)(self.ptr, reason)
    }

    pub fn restart_ee(&self, finished_gc: bool) {
        (self.vtable().RestartEE)(self.ptr, finished_gc)
    }

    pub fn scan_roots<F>(&self, generation: i32, max_gen: i32, promotion: bool, is_bgc: bool, is_concurrent: bool, mut callback: F) where F: FnMut(&mut ObjectRef, &ScanContext, ScanFlags) {
        (self.vtable().BeforeGcScanRoots)(self.ptr, generation, is_bgc, is_concurrent);

        let mut sc = ScanContext::default();
        sc.promotion = promotion;
        sc._unused1 = &raw mut callback as usize;

        extern "system" fn scan_callback<F>(ppObject: *mut ObjectRef, sc: *const ScanContext, flags: ScanFlags) where F: FnMut(&mut ObjectRef, &ScanContext, ScanFlags) {
            unsafe {
                let action = (*sc)._unused1 as *mut F;
                (*action)(&mut *ppObject, &*sc, flags);
            }
        }
        (self.vtable().GcScanRoots)(self.ptr, scan_callback::<F>, generation, max_gen, &sc);

        (self.vtable().AfterGcScanRoots)(self.ptr, generation, is_bgc, &sc);
    }

    pub fn gc_start_work(&self, generation: i32, max_gen: i32) {
        (self.vtable().GcStartWork)(self.ptr, generation, max_gen)
    }

    pub fn gc_done(&self, generation: i32) {
        (self.vtable().GcDone)(self.ptr, generation)
    }

    pub fn get_allocate_context(&self) -> &'static gc_alloc_context {
        unsafe { &*(self.vtable().GetAllocContext)(self.ptr) }
    }

    pub fn for_each_alloc_context<F>(&self, mut action: F) where F: FnMut(&gc_alloc_context) {
        extern "system" fn callback<F>(alloc_context: *const gc_alloc_context, param: usize) where F: FnMut(&gc_alloc_context) {
            unsafe {
                let action = param as *mut F;
                (*action)(&*alloc_context);
            }
        }
        (self.vtable().GcEnumAllocContexts)(self.ptr, callback::<F>, &raw mut action as usize)
    }

    pub fn stomp_write_barrier(&self, args: &WriteBarrierParameters) {
        (self.vtable().StompWriteBarrier)(self.ptr, args)
    }
}
