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

pub type ObjectRef = *mut Object;

impl Object {
    pub fn has_component_size(&self) -> bool {
        let mt = unsafe { &*self.method_table };
        mt.flags_high & 0x8000 != 0
    }

    pub fn total_size(&self) -> u32 {
        let mt = unsafe { &*self.method_table };
        mt.base_size + if self.has_component_size() { mt.component_size as u32 * self.component_count } else { 0 }
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Default)]
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
