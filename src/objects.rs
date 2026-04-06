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

pub type ObjectRef = *mut Object;

fn align_to_ptr(size: u32) -> usize {
    let mask = size_of::<usize>() - 1;
    (size as usize + mask) & !mask
}

impl Object {
    const HAS_COMPONENT_SIZE: u16 = 0x8000;
    const HAS_GC_POINTERS: u16 = 0x0100;

    pub fn has_component_size(&self) -> bool {
        let mt = unsafe { &*self.method_table };
        mt.flags_high & Self::HAS_COMPONENT_SIZE != 0
    }

    #[inline]
    pub fn total_size(&self) -> u32 {
        let mt = unsafe { &*self.method_table };
        mt.base_size + if self.has_component_size() { mt.component_size as u32 * self.component_count } else { 0 }
    }

    #[inline]
    pub fn total_size_aligned(&self) -> usize {
        align_to_ptr(self.total_size())
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
