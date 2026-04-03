use crate::ObjectRef;
use std::ffi::c_void;

#[repr(C)]
pub struct gc_alloc_context {
    alloc_ptr: usize,
    alloc_limit: usize,
    alloc_bytes: i64,
    alloc_bytes_uoh: i64,
    gc_reserved_1: *const c_void,
    gc_reserved_2: *const c_void,
    alloc_count: i32
}

#[repr(C)]
pub struct IGCHeap {
    vptr: *const IGCHeapVTable,
}

#[repr(C)]
pub struct IGCHeapVTable {
    // Hosting APIs
    IsValidSegmentSize: unsafe extern "system" fn(this: *mut IGCHeap, size: isize) -> bool,
    IsValidGen0MaxSize: unsafe extern "system" fn(this: *mut IGCHeap, size: isize) -> bool,
    GetValidSegmentSize: unsafe extern "system" fn(this: *mut IGCHeap, large_seg: bool) -> isize,
    SetReservedVMLimit: unsafe extern "system" fn(this: *mut IGCHeap, size: isize),
    // Concurrent GC
    WaitUntilConcurrentGCComplete: unsafe extern "system" fn(this: *mut IGCHeap),
    IsConcurrentGCInProgress: unsafe extern "system" fn(this: *mut IGCHeap) -> bool,
    TemporaryEnableConcurrentGC: unsafe extern "system" fn(this: *mut IGCHeap),
    TemporaryDisableConcurrentGC: unsafe extern "system" fn(this: *mut IGCHeap),
    IsConcurrentGCEnabled: unsafe extern "system" fn(this: *mut IGCHeap) -> bool,
    WaitUntilConcurrentGCCompleteAsync: unsafe extern "system" fn(this: *mut IGCHeap, millisecondsTimeout: i32) -> u32,
    // Finalization
    GetNumberOfFinalizable: unsafe extern "system" fn(this: *mut IGCHeap) -> isize,
    GetNextFinalizable: unsafe extern "system" fn(this: *mut IGCHeap) -> ObjectRef,
    // BCL rountines
    GetMemoryInfo: unsafe extern "system" fn(this: *mut IGCHeap,
        highMemLoadThresholdBytes: *mut u64,
        totalAvailableMemoryBytes: *mut u64,
        lastRecordedMemLoadBytes: *mut u64,
        lastRecordedHeapSizeBytes: *mut u64,
        lastRecordedFragmentationBytes: *mut u64,
        totalCommittedBytes: *mut u64,
        promotedBytes: *mut u64,
        pinnedObjectCount: *mut u64,
        finalizationPendingCount: *mut u64,
        index: *mut u64,
        generation: *mut u32,
        isCompaction: *mut bool,
        isConcurrent: *mut bool,
        genInfoRaw: *mut u64,
        pauseInfoRaw: *mut u64,
        kind: i32
    ),
    GetMemoryLoad: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    GetGcLatencyMode: unsafe extern "system" fn(this: *mut IGCHeap) -> i32,
    SetGcLatencyMode: unsafe extern "system" fn(this: *mut IGCHeap, newLatencyMode: i32) -> i32,
    GetLOHCompactionMode: unsafe extern "system" fn(this: *mut IGCHeap) -> i32,
    SetLOHCompactionMode: unsafe extern "system" fn(this: *mut IGCHeap, newLOHCompactionMode: i32),
    RegisterForFullGCNotification: unsafe extern "system" fn(this: *mut IGCHeap, gen2Percentage: u32, lohPercentage: u32) -> bool,
    CancelFullGCNotification: unsafe extern "system" fn(this: *mut IGCHeap) -> bool,
    WaitForFullGCApproach: unsafe extern "system" fn(this: *mut IGCHeap, millisecondsTimeout: i32) -> i32,
    WaitForFullGCComplete: unsafe extern "system" fn(this: *mut IGCHeap, millisecondsTimeout: i32) -> i32,
    WhichGeneration: unsafe extern "system" fn(this: *mut IGCHeap, obj: ObjectRef) -> i32,
    CollectionCount: unsafe extern "system" fn(this: *mut IGCHeap, generation: i32, get_bgc_fgc_count: i32) -> i32,
    StartNoGCRegion: unsafe extern "system" fn(this: *mut IGCHeap, totalSize: u64, lohSizeKnown: bool, lohSize: u64, disallowFullBlockingGC: bool) -> i32,
    EndNoGCRegion: unsafe extern "system" fn(this: *mut IGCHeap) -> i32,
    GetTotalBytesInUse: unsafe extern "system" fn(this: *mut IGCHeap) -> isize,
    GetTotalAllocatedBytes: unsafe extern "system" fn(this: *mut IGCHeap) -> i64,
    GarbageCollect: unsafe extern "system" fn(this: *mut IGCHeap, generation: i32, low_memory_p: bool, mode: i32) -> u32,
    GetMaxGeneration: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    SetFinalizationRun: unsafe extern "system" fn(this: *mut IGCHeap, obj: ObjectRef),
    RegisterForFinalization: unsafe extern "system" fn(this: *mut IGCHeap, generation: i32, obj: ObjectRef) -> bool,
    GetLastGCPercentTimeInGC: unsafe extern "system" fn(this: *mut IGCHeap) -> i32,
    GetLastGCGenerationSize: unsafe extern "system" fn(this: *mut IGCHeap, generation: i32) -> isize,
    // Miscellaneous routines used by the VM
    Initialize: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    IsPromoted: unsafe extern "system" fn(this: *mut IGCHeap, obj: ObjectRef) -> bool,
    IsHeapPointer: unsafe extern "system" fn(this: *mut IGCHeap, obj: *const c_void, small_heap_only: bool) -> bool,
    GetCondemnedGeneration: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    IsGCInProgressHelper: unsafe extern "system" fn(this: *mut IGCHeap, bConsiderGCStart: bool) -> bool,
    GetGcCount: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    IsThreadUsingAllocationContextHeap: unsafe extern "system" fn(this: *mut IGCHeap, acontext: *mut gc_alloc_context, thread_number: i32) -> bool,
    IsEphemeral: unsafe extern "system" fn(this: *mut IGCHeap, acontext: ObjectRef) -> bool,
    WaitUntilGCComplete: unsafe extern "system" fn(this: *mut IGCHeap, bConsiderGCStart: bool) -> u32,
    FixAllocContext: unsafe extern "system" fn(this: *mut IGCHeap, arg: usize, heap: usize),
    GetCurrentObjSize: unsafe extern "system" fn(this: *mut IGCHeap) -> isize,
    SetGCInProgress: unsafe extern "system" fn(this: *mut IGCHeap, fInProgress: bool),
    RuntimeStructuresValid: unsafe extern "system" fn(this: *mut IGCHeap) -> bool,
    SetSuspensionPending: unsafe extern "system" fn(this: *mut IGCHeap, fSuspensionPending: bool),
    SetYieldProcessorScalingFactor: unsafe extern "system" fn(this: *mut IGCHeap, yieldProcessorScalingFactor: f32),
    Shutdown: unsafe extern "system" fn(this: *mut IGCHeap),
    // Add/RemoveMemoryPressure support routines.
    GetLastGCStartTime: unsafe extern "system" fn (this: *mut IGCHeap, generation: i32) -> isize,
    GetLastGCDuration: unsafe extern "system" fn (this: *mut IGCHeap, generation: i32) -> isize,
    GetNow: unsafe extern "system" fn (this: *mut IGCHeap, generation: i32) -> isize,
    // Allocation routines
    Alloc: unsafe extern "system" fn (this: *mut IGCHeap, acontext: *mut gc_alloc_context, size: isize, flags: u32) -> ObjectRef,
    PublishObject: unsafe extern "system" fn (this: *mut IGCHeap, obj: usize) -> ObjectRef,
    SetWaitForGCEvent: unsafe extern "system" fn (this: *mut IGCHeap),
    ResetWaitForGCEvent: unsafe extern "system" fn (this: *mut IGCHeap),
}
