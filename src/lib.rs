use std::ffi::c_char;
use crate::gcinterface::{GcDescVars, IGCHandleManager, IGCHeap, IGCToCLR};

mod gcinterface;

#[allow(nonstandard_style)]
#[unsafe(no_mangle)]
pub extern "C" fn GC_Initialize(
    _clrToGC: *const IGCToCLR,
    _gcHeap: *mut *const IGCHeap,
    _gcHandleManager: *mut *const IGCHandleManager,
    _gcDescVars: *mut GcDescVars) -> u32 {
    println!("GC_Initialize!");
    0x80004005
}

#[allow(nonstandard_style)]
#[repr(C)]
pub struct VersionInfo {
    MajorVersion: u32,
    MinorVersion: u32,
    BuildVersion: u32,
    Name: *const c_char,
}

#[unsafe(no_mangle)]
pub extern "C" fn GC_VersionInfo(info: *mut VersionInfo) {
    println!("GC_VersionInfo!");
    unsafe {
        (*info).MajorVersion = 5;
        (*info).MinorVersion = 8;
        (*info).BuildVersion = 0;
        (*info).Name = b"Rust GC\0".as_ptr() as *const c_char;
    }
}
