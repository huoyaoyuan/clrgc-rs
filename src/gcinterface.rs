#[repr(C)]
pub struct IGCToCLR {

}

#[repr(C)]
pub struct IGCHeap {

}

#[repr(C)]
pub struct IGCHandleManager {

}

#[repr(C)]
pub struct GcDescVars {
    major_version_number: u8,
    minor_version_number: u8,
    generation_size: isize,
    total_generation_count: isize,
}
