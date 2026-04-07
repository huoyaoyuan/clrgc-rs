use crate::gc::RustGc;
use crate::gcinterface::gc_to_clr::{WriteBarrierOp, WriteBarrierParameters};
use crate::objects::ObjectRef;

#[repr(C)]
pub struct gc_alloc_context {
    alloc_ptr: usize,
    alloc_limit: usize,
    alloc_bytes: i64,
    alloc_bytes_uoh: i64,
    gc_reserved_1: usize,
    gc_reserved_2: usize,
    alloc_count: i32
}

#[repr(C)]
pub struct IGCHeap {
    vptr: *const IGCHeapVTable,
    gc: *mut RustGc,
}

type DummyFunc = extern "system" fn() -> usize;
extern "system" fn nop() -> usize { 0 }
extern "system" fn nop_ret_non_null() -> usize { 1 }

#[repr(C)]
pub struct IGCHeapVTable {
    // Hosting APIs
    hosting: [DummyFunc; 4],
    // IsValidSegmentSize: unsafe extern "system" fn(this: *mut IGCHeap, size: isize) -> bool,
    // IsValidGen0MaxSize: unsafe extern "system" fn(this: *mut IGCHeap, size: isize) -> bool,
    // GetValidSegmentSize: unsafe extern "system" fn(this: *mut IGCHeap, large_seg: bool) -> isize,
    // SetReservedVMLimit: unsafe extern "system" fn(this: *mut IGCHeap, size: isize),
    // Concurrent GC
    concurrent: [DummyFunc; 6],
    // WaitUntilConcurrentGCComplete: unsafe extern "system" fn(this: *mut IGCHeap),
    // IsConcurrentGCInProgress: unsafe extern "system" fn(this: *mut IGCHeap) -> bool,
    // TemporaryEnableConcurrentGC: unsafe extern "system" fn(this: *mut IGCHeap),
    // TemporaryDisableConcurrentGC: unsafe extern "system" fn(this: *mut IGCHeap),
    // IsConcurrentGCEnabled: unsafe extern "system" fn(this: *mut IGCHeap) -> bool,
    // WaitUntilConcurrentGCCompleteAsync: unsafe extern "system" fn(this: *mut IGCHeap, millisecondsTimeout: i32) -> u32,
    // Finalization
    finalization: [DummyFunc; 2],
    // GetNumberOfFinalizable: unsafe extern "system" fn(this: *mut IGCHeap) -> isize,
    // GetNextFinalizable: unsafe extern "system" fn(this: *mut IGCHeap) -> ObjectRef,
    // BCL rountines
    bcl: [DummyFunc; 16],
    // GetMemoryInfo: unsafe extern "system" fn(this: *mut IGCHeap,
    //     highMemLoadThresholdBytes: *mut u64,
    //     totalAvailableMemoryBytes: *mut u64,
    //     lastRecordedMemLoadBytes: *mut u64,
    //     lastRecordedHeapSizeBytes: *mut u64,
    //     lastRecordedFragmentationBytes: *mut u64,
    //     totalCommittedBytes: *mut u64,
    //     promotedBytes: *mut u64,
    //     pinnedObjectCount: *mut u64,
    //     finalizationPendingCount: *mut u64,
    //     index: *mut u64,
    //     generation: *mut u32,
    //     isCompaction: *mut bool,
    //     isConcurrent: *mut bool,
    //     genInfoRaw: *mut u64,
    //     pauseInfoRaw: *mut u64,
    //     kind: i32
    // ),
    // GetMemoryLoad: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    // GetGcLatencyMode: unsafe extern "system" fn(this: *mut IGCHeap) -> i32,
    // SetGcLatencyMode: unsafe extern "system" fn(this: *mut IGCHeap, newLatencyMode: i32) -> i32,
    // GetLOHCompactionMode: unsafe extern "system" fn(this: *mut IGCHeap) -> i32,
    // SetLOHCompactionMode: unsafe extern "system" fn(this: *mut IGCHeap, newLOHCompactionMode: i32),
    // RegisterForFullGCNotification: unsafe extern "system" fn(this: *mut IGCHeap, gen2Percentage: u32, lohPercentage: u32) -> bool,
    // CancelFullGCNotification: unsafe extern "system" fn(this: *mut IGCHeap) -> bool,
    // WaitForFullGCApproach: unsafe extern "system" fn(this: *mut IGCHeap, millisecondsTimeout: i32) -> i32,
    // WaitForFullGCComplete: unsafe extern "system" fn(this: *mut IGCHeap, millisecondsTimeout: i32) -> i32,
    // WhichGeneration: unsafe extern "system" fn(this: *mut IGCHeap, obj: ObjectRef) -> i32,
    // CollectionCount: unsafe extern "system" fn(this: *mut IGCHeap, generation: i32, get_bgc_fgc_count: i32) -> i32,
    // StartNoGCRegion: unsafe extern "system" fn(this: *mut IGCHeap, totalSize: u64, lohSizeKnown: bool, lohSize: u64, disallowFullBlockingGC: bool) -> i32,
    // EndNoGCRegion: unsafe extern "system" fn(this: *mut IGCHeap) -> i32,
    // GetTotalBytesInUse: unsafe extern "system" fn(this: *mut IGCHeap) -> isize,
    // GetTotalAllocatedBytes: unsafe extern "system" fn(this: *mut IGCHeap) -> i64,
    GarbageCollect: extern "system" fn(this: *mut IGCHeap, generation: i32, low_memory_p: bool, mode: i32) -> u32,
    bcl2: [DummyFunc; 5],
    // GetMaxGeneration: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    // SetFinalizationRun: unsafe extern "system" fn(this: *mut IGCHeap, obj: ObjectRef),
    // RegisterForFinalization: unsafe extern "system" fn(this: *mut IGCHeap, generation: i32, obj: ObjectRef) -> bool,
    // GetLastGCPercentTimeInGC: unsafe extern "system" fn(this: *mut IGCHeap) -> i32,
    // GetLastGCGenerationSize: unsafe extern "system" fn(this: *mut IGCHeap, generation: i32) -> isize,
    // Miscellaneous routines used by the VM
    Initialize: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    vm1: [DummyFunc; 7],
    vm2: [DummyFunc; 8],
    // IsPromoted: unsafe extern "system" fn(this: *mut IGCHeap, obj: ObjectRef) -> bool,
    // IsHeapPointer: unsafe extern "system" fn(this: *mut IGCHeap, obj: *const c_void, small_heap_only: bool) -> bool,
    // GetCondemnedGeneration: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    // IsGCInProgressHelper: unsafe extern "system" fn(this: *mut IGCHeap, bConsiderGCStart: bool) -> bool,
    // GetGcCount: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    // IsThreadUsingAllocationContextHeap: unsafe extern "system" fn(this: *mut IGCHeap, acontext: *mut gc_alloc_context, thread_number: i32) -> bool,
    // IsEphemeral: unsafe extern "system" fn(this: *mut IGCHeap, acontext: ObjectRef) -> bool,
    // WaitUntilGCComplete: unsafe extern "system" fn(this: *mut IGCHeap, bConsiderGCStart: bool) -> u32,
    // FixAllocContext: unsafe extern "system" fn(this: *mut IGCHeap, arg: usize, heap: usize),
    // GetCurrentObjSize: unsafe extern "system" fn(this: *mut IGCHeap) -> isize,
    // SetGCInProgress: unsafe extern "system" fn(this: *mut IGCHeap, fInProgress: bool),
    // RuntimeStructuresValid: unsafe extern "system" fn(this: *mut IGCHeap) -> bool,
    // SetSuspensionPending: unsafe extern "system" fn(this: *mut IGCHeap, fSuspensionPending: bool),
    // SetYieldProcessorScalingFactor: unsafe extern "system" fn(this: *mut IGCHeap, yieldProcessorScalingFactor: f32),
    // Shutdown: unsafe extern "system" fn(this: *mut IGCHeap),
    // Add/RemoveMemoryPressure support routines.
    timing: [DummyFunc; 3],
    // GetLastGCStartTime: unsafe extern "system" fn (this: *mut IGCHeap, generation: i32) -> isize,
    // GetLastGCDuration: unsafe extern "system" fn (this: *mut IGCHeap, generation: i32) -> isize,
    // GetNow: unsafe extern "system" fn (this: *mut IGCHeap, generation: i32) -> isize,
    // Allocation routines
    Alloc: extern "system" fn (this: *mut IGCHeap, acontext: *mut gc_alloc_context, size: usize, flags: u32) -> ObjectRef,
    alloc: [DummyFunc; 3],
    // PublishObject: unsafe extern "system" fn (this: *mut IGCHeap, obj: usize),
    // SetWaitForGCEvent: unsafe extern "system" fn (this: *mut IGCHeap),
    // ResetWaitForGCEvent: unsafe extern "system" fn (this: *mut IGCHeap),
    verification: [DummyFunc; 4],
    profiling: [DummyFunc; 11],
    stress_heap: DummyFunc,
    frozen: [DummyFunc; 3],
    more: [DummyFunc; 13],
}

fn get_gc(this: *mut IGCHeap) -> &'static mut RustGc {
    unsafe { &mut *(*this).gc }
}

extern "system" fn GCHeap_GarbageCollect(this: *mut IGCHeap, generation: i32, _low_memory_p: bool, _mode: i32) -> u32 {
    get_gc(this).do_collect(generation);
    0
}

extern "system" fn GCHeap_Initialize(this: *mut IGCHeap) -> u32 {
    println!("GCHeap::Initialize");

    let mut write_barrier_args = WriteBarrierParameters::default();
    write_barrier_args.operation = WriteBarrierOp::Initialize;
    write_barrier_args.is_runtime_suspended = true;
    write_barrier_args.ephemeral_low = usize::MAX;
    get_gc(this).clr.stomp_write_barrier(&write_barrier_args);

    0
}

extern "system" fn GCHeap_Alloc(this: *mut IGCHeap, acontext: *mut gc_alloc_context, size: usize, _flags: u32) -> ObjectRef {
    let context = unsafe { &mut *acontext };
    let obj = context.alloc_ptr as ObjectRef;
    let new_ptr = context.alloc_ptr + size;
    if new_ptr < context.alloc_limit {
        context.alloc_ptr = new_ptr;
        obj
    } else {
        if context.alloc_limit != 0 {
            get_gc(this).complete_segment(context.alloc_limit);
        }

        // Trigger a GC for each new segment
        get_gc(this).do_collect(0);

        let segment = get_gc(this).add_segment(size);
        println!("Allocated new segment at {:016x}, Length {}", segment.start as usize, unsafe { segment.end.byte_offset_from(segment.start) });
        let obj_ptr = segment.start.wrapping_add(1);
        context.alloc_ptr = obj_ptr as usize + size;
        context.alloc_limit = segment.end as usize;
        obj_ptr as ObjectRef
    }
}

const GCHeap_vtable : IGCHeapVTable = IGCHeapVTable {
    hosting: [nop; 4],
    concurrent: [nop; 6],
    finalization: [nop; 2],
    bcl: [nop; 16],
    GarbageCollect: GCHeap_GarbageCollect,
    bcl2: [nop; 5],
    Initialize: GCHeap_Initialize,
    vm1: [nop_ret_non_null; 7],
    vm2: [nop; 8],
    timing: [nop; 3],
    Alloc: GCHeap_Alloc,
    alloc: [nop; 3],
    verification: [nop; 4],
    profiling: [nop; 11],
    stress_heap: nop,
    frozen: [nop_ret_non_null, nop, nop],
    more: [nop; 13],
};

impl IGCHeap {
    pub fn new(gc: *mut RustGc) -> Self {
        Self {
            vptr: &GCHeap_vtable,
            gc,
        }
    }
}
