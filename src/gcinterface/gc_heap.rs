use std::ptr::null_mut;
use bitflags::bitflags;

use super::*;
use crate::gc::RustGc;
use crate::objects::*;

/// The core allocation structure, representing a continuous region for allocation.
/// 
/// Most allocations of small objects are done by CLR helper code without calling into GC.
/// Every threads gets a thread-local allocation context, and consults GC when it's full.
#[repr(C)]
pub struct gc_alloc_context {
    /// The address of next allocated object.
    pub alloc_ptr: usize,
    /// The upper limit (exclusive) of the allocation context.
    pub alloc_limit: usize,
    alloc_bytes: i64,
    alloc_bytes_uoh: i64,
    gc_reserved_1: usize,
    gc_reserved_2: usize,
    alloc_count: i32,
}

bitflags! {
    #[repr(transparent)]
    pub struct AllocFlags : i32 {
        const Finalizable = 1;
        const ContainsReference = 2;
        const Align8Bias = 4;
        const Align8 = 8;
        const ZeroingOptional = 16;
        const LargeObjectHeap = 32;
        const PinnedObjectHeap = 64;
        const UserOldHeap = Self::LargeObjectHeap.bits() | Self::PinnedObjectHeap.bits();
    }
}

#[repr(C)]
pub struct IGCHeap {
    vptr: *const IGCHeapVTable,
    gc: *mut RustGc,
}

// The tricky nop functions that can fit most signatures.
// Notably it won't work on x86 stdcall, where callee cleans the stack.
type DummyFunc = extern "system" fn() -> usize;
extern "system" fn nop() -> usize { 0 }
extern "system" fn nop_ret_non_null() -> usize { 1 }

/// The central interface for the GC.
/// 
/// Although many of the methods are not required for basic functionality, several methods must be set up, otherwise CLR will crash for hello world:
/// - [`IGCHeapVTable::GetNextFinalizable`] must not return a wild pointer. Return `null` for prototype implementation.
/// - `GetExtraWorkForFinalization`([`IGCHeapVTable::more`]`[8]`) must not return a wild pointer.
/// - [`IGCHeapVTable::RegisterForFinalization`] must return true when any finalization support is set up, from the `Gen2GCCallback` object in BCL.
/// - `IsEphemeral`([`IGCHeapVTable::vm1`]`[6]`) should return `true`, otherwise some roots won't be reported when a collection is considered ephemeral by CLR.
/// - `RegisterFrozenSegment`([`IGCHeapVTable::frozen`]`[0]`) must return non-zero handle, otherwise CLR will fail to start.
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
    GetNumberOfFinalizable: extern "system" fn(this: *mut IGCHeap) -> isize,
    GetNextFinalizable: extern "system" fn(this: *mut IGCHeap) -> ObjectRef,
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
    GetMaxGeneration: DummyFunc,
    // GetMaxGeneration: unsafe extern "system" fn(this: *mut IGCHeap) -> u32,
    SetFinalizationRun: extern "system" fn(this: *mut IGCHeap, obj: ObjectRef),
    RegisterForFinalization: extern "system" fn(this: *mut IGCHeap, generation: i32, obj: ObjectRef) -> bool,
    bcl2: [DummyFunc; 2],
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
    Alloc: extern "system" fn (this: *mut IGCHeap, acontext: *mut gc_alloc_context, size: usize, flags: AllocFlags) -> ObjectRef,
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

extern "system" fn GCHeap_GetNumberOfFinalizable(_: *mut IGCHeap) -> isize {
    // Not really used in modern CLR
    unimplemented!()
}

extern "system" fn GCHeap_GetNextFinalizable(this: *mut IGCHeap) -> ObjectRef {
    let f = get_gc(this).pop_finalizable();
    f.unwrap_or(null_mut())
}

extern "system" fn GCHeap_GarbageCollect(this: *mut IGCHeap, generation: i32, _low_memory_p: bool, _mode: i32) -> u32 {
    get_gc(this).do_collect(generation);
    0
}

extern "system" fn GCHeap_SetFinalizationRun(_: *mut IGCHeap, obj: ObjectRef) {
    unsafe { &mut *obj }.set_finalizer_run(true);
}

extern "system" fn GCHeap_RegisterForFinalization(this: *mut IGCHeap, _: i32, obj: ObjectRef) -> bool {
    get_gc(this).reregister_finalization(obj);
    true
}

/// The initialization method is called by CLR when it has initialized other components.
extern "system" fn GCHeap_Initialize(this: *mut IGCHeap) -> u32 {
    println!("GCHeap::Initialize");

    unsafe {
        FREE_MT = get_gc(this).clr.get_free_methodtable();
        if FREE_MT.is_null()
            || (*FREE_MT).component_size != 1
            || (*FREE_MT).base_size != Object::BASE_SIZE as u32 {
            return 80004005;
        }
    }

    // Toggle the write barrier code to skip writing the card table.
    // The write barrier code hard codes generational behavior and the card table structure.

    // Card table is used when writing references from older generation to ephemeral generation.
    // Setting the ephemeral generation range to empty effectively disables the card table.
    let mut write_barrier_args = WriteBarrierParameters::default();
    write_barrier_args.operation = WriteBarrierOp::Initialize;
    write_barrier_args.is_runtime_suspended = true;
    write_barrier_args.ephemeral_low = usize::MAX;
    get_gc(this).clr.stomp_write_barrier(&write_barrier_args);

    0
}

/// The core allocation method invoked by CLR or managed code.
/// It's invoked every time when an allocation context is full, or an object needs special flags (e.g. pinned, finalizable, large object).
/// 
/// GC is only asked for a space of the total size. [`Object::method_table`] and [`Object::component_count`] will be filled by CLR code.
extern "system" fn GCHeap_Alloc(this: *mut IGCHeap, acontext: *mut gc_alloc_context, size: usize, flags: AllocFlags) -> ObjectRef {
    if flags.intersects(AllocFlags::Align8 | AllocFlags::Align8Bias) {
        // Align8 and Align8Bias are used for objects requiring 8-byte alignment on 32-bit platforms, namely double arrays.
        // Align8 requires the object to be 8-byte aligned, and Align8Bias requires the object to be 4-byte aligned but not 8-byte aligned.
        // Since we are not interested to support 32-bit platforms, just reject such flags.
        unimplemented!()
    }

    // The size parameter represents on-heap size, which includes the object header and method table.
    // For variable-sized objects, namely string and byte[], the size is not aligned to pointer size by the caller.
    let size = align_to_ptr(size);

    let context = unsafe { &mut *acontext };
    let obj = context.alloc_ptr as ObjectRef;
    // Matches the check in CLR helper code. alloc_ptr + size can potentially overflow.
    if context.alloc_limit - context.alloc_ptr >= size {
        // This is actually the rare path. Most small objects are allocated by CLR helper code.
        context.alloc_ptr = context.alloc_ptr + size;
        obj
    } else {
        // Trigger a GC for each new segment
        get_gc(this).do_collect(0);

        // Most flags are optimizational hint and can be ignored. The PinnedObjectHeap flag means the object is pinned permanently and requires special handling.
        let segment = get_gc(this).add_segment(size, flags.contains(AllocFlags::PinnedObjectHeap));
        println!("Allocated new segment at {:016x}-{:016x}, Length {}", segment.start as usize, segment.end as usize, unsafe { segment.end.byte_offset_from(segment.start) });
        // Leave a pointer space for object header.
        let obj_ptr = segment.start.wrapping_add(1);
        context.alloc_ptr = obj_ptr as usize + size;
        context.alloc_limit = segment.end as usize;
        // We must ensure alloc_limit >= alloc_ptr, since the CLR helper code does unsigned substraction for alloc_limit - alloc_ptr >= size.
        debug_assert!(context.alloc_limit >= context.alloc_ptr);
        debug_assert!((obj_ptr as ObjectRef).is_aligned());
        obj_ptr as ObjectRef
    }
}

static GCHeap_vtable: IGCHeapVTable = IGCHeapVTable {
    hosting: [nop; 4],
    concurrent: [nop; 6],
    GetNumberOfFinalizable: GCHeap_GetNumberOfFinalizable,
    GetNextFinalizable: GCHeap_GetNextFinalizable,
    bcl: [nop; 16],
    GarbageCollect: GCHeap_GarbageCollect,
    GetMaxGeneration: nop,
    SetFinalizationRun: GCHeap_SetFinalizationRun,
    RegisterForFinalization: GCHeap_RegisterForFinalization,
    bcl2: [nop_ret_non_null; 2],
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
