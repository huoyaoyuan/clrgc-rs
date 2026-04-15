use bitflags::bitflags;

#[repr(C)]
pub struct Object {
    pub method_table: *const MethodTable,
    pub component_count: u32,
}

#[repr(C)]
pub struct MethodTable {
    pub component_size: u16,
    pub flags_high: u16,
    pub base_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct GCDescSeries {
    pub size: isize,
    pub offset: usize,
}

#[cfg(target_pointer_width = "64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct ValSeriesItem {
    pub pointers: u32,
    pub skip: u32,
}

#[cfg(target_pointer_width = "32")]
#[repr(C)]
#[derive(Clone, Copy)]
struct ValSeriesItem {
    pub pointers: u16,
    pub skip: u16,
}

bitflags! {
    #[repr(transparent)]
    struct ObjectHeader : u32 {
        const BIT_SBLK_FINALIZER_RUN = 0x40000000;
    }
}

pub type ObjectRef = *mut Object;

pub fn align_to_ptr(size: usize) -> usize {
    let mask = size_of::<usize>() - 1;
    (size + mask) & !mask
}

pub static mut FREE_MT: *const MethodTable = std::ptr::null();

impl Object {
    pub const HAS_COMPONENT_SIZE: u16 = 0x8000;
    pub const HAS_GC_POINTERS: u16 = 0x0100;
    pub const HAS_FINALIZER: u16 = 0x0010;
    pub const BASE_SIZE: usize = 3 * size_of::<usize>();

    #[inline]
    pub fn has_component_size(&self) -> bool {
        let mt = unsafe { &*self.method_table };
        mt.flags_high & Self::HAS_COMPONENT_SIZE != 0
    }

    #[inline]
    pub fn is_finalizable(&self) -> bool {
        let mt = unsafe { &*self.method_table };
        mt.flags_high & Self::HAS_FINALIZER != 0
    }

    #[inline]
    pub fn get_finalizer_run(&mut self) -> bool {
        let header = (self as ObjectRef as *mut usize).wrapping_sub(1) as *const ObjectHeader;
        unsafe { (*header).contains(ObjectHeader::BIT_SBLK_FINALIZER_RUN) }
    }

    #[inline]
    pub fn set_finalizer_run(&mut self, value: bool) {
        let header = (self as ObjectRef as *mut usize).wrapping_sub(1) as *mut ObjectHeader;
        unsafe { (*header).set(ObjectHeader::BIT_SBLK_FINALIZER_RUN, value); }
    }

    #[inline]
    pub fn needs_finalization(&mut self) -> bool {
        self.is_finalizable() && !self.get_finalizer_run()
    }

    #[inline]
    pub fn total_size(&self) -> u32 {
        let mt = unsafe { &*self.method_table };
        mt.base_size + if self.has_component_size() { mt.component_size as u32 * self.component_count } else { 0 }
    }

    #[inline]
    pub fn total_size_aligned(&self) -> usize {
        align_to_ptr(self.total_size() as usize)
    }

    pub fn for_each_obj_ref<F: FnMut(&mut ObjectRef)>(&mut self, mut f: F) {
        unsafe {
            if (*self.method_table).flags_high & Self::HAS_GC_POINTERS == 0 {
                return;
            }
            let base_ptr = (self.method_table as *const isize).sub(1);
            let series_count = *base_ptr;

            if series_count >= 0 {
                for s in 1..series_count + 1 {
                    let gc_desc_base = base_ptr as *const GCDescSeries;
                    let series = *gc_desc_base.sub(s as usize);
                    let field_count = (self.total_size_aligned() as isize + series.size) as usize / size_of::<usize>();
                    let series_ptr = (&raw mut self.method_table as *mut ObjectRef).byte_add(series.offset);

                    for i in 0..field_count {
                        f(&mut *series_ptr.add(i));
                    }
                }
            } else {
                let component_size = (*self.method_table).component_size as usize;
                let offset = *base_ptr.sub(1);
                let val_series_base = base_ptr.sub(2) as *const ValSeriesItem;
                let elements_base = (&raw mut self.method_table as *mut ObjectRef).byte_offset(offset);

                for e in 0..self.component_count as usize {
                    let mut element_ptr = elements_base.byte_add(e * component_size);
                    for s in 0..(-series_count as usize) {
                        let item = *val_series_base.sub(s);
                        for i in 0..item.pointers as usize {
                            f(&mut *element_ptr.add(i));
                        }
                        element_ptr = element_ptr.add(item.pointers as usize).byte_add(item.skip as usize);
                    }
                }
            }
        }
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum HandleType {
    #[default]
    Short = 0,
    ShortRecurrsion = 1,
    Strong = 2,
    Pinned = 3,
    Dependent = 6,
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct GcHandle {
    pub object: ObjectRef,
    pub extra_or_secondary: usize,
    pub handle_type: HandleType,
}

pub type ObjectHandle = *mut GcHandle;
