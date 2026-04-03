use std::ffi::c_char;
use crate::{gc::RustGc, gcinterface::{GcDescVars, IGCHandleManager, IGCHeap, IGCToCLR}};

mod gc;
mod gcinterface;

unsafe fn heap_alloc<T>(value: T) -> *mut T {
    let b = Box::new(value);
    Box::into_raw(b)
}

#[allow(nonstandard_style)]
#[unsafe(no_mangle)]
pub extern "C" fn GC_Initialize(
    _clrToGC: *const IGCToCLR,
    _gcHeap: *mut *const IGCHeap,
    gcHandleManager: *mut *const IGCHandleManager,
    _gcDescVars: *mut GcDescVars) -> u32 {
    println!("GC_Initialize!");

    unsafe {
        let gc = heap_alloc(RustGc::new());
        *gcHandleManager = heap_alloc(IGCHandleManager::new(gc))
    }
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
