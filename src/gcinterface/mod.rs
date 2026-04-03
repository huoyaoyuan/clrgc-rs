#![allow(nonstandard_style)]

mod gc_heap;
mod gc_to_clr;
mod handle_manager;

pub use gc_heap::IGCHeap;
pub use gc_to_clr::IGCToCLR;
pub use handle_manager::IGCHandleManager;

#[repr(C)]
pub struct Object {

}

#[repr(C)]
pub struct GcDescVars {
    major_version_number: u8,
    minor_version_number: u8,
    generation_size: isize,
    total_generation_count: isize,
}
