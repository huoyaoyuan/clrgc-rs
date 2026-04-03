use super::Object;

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
    SuspendEE: unsafe extern "system" fn(this: *const IGCToCLR, reason: SUSPEND_REASON),
    RestartEE: unsafe extern "system" fn(this: *const IGCToCLR, bFinishedGC: bool),
    GcScanRoots: unsafe extern "system" fn(this: *const IGCToCLR, func: promote_func, condemned: i32, max_gen: i32, sc: *const ScanContext),
    GcStartWork: unsafe extern "system" fn(this: *const IGCToCLR, condemned: i32, max_gen: i32),
    BeforeGcScanRoots: unsafe extern "system" fn(this: *const IGCToCLR, condemned: i32, is_bgc: bool, is_concurrent: bool),
    AfterGcScanRoots: unsafe extern "system" fn(this: *const IGCToCLR, condemned: i32, is_bgc: bool, sc: *const ScanContext),
    GcDone: unsafe extern "system" fn(this: *const IGCToCLR, condemned: i32),
}
