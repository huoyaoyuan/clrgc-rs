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

#[repr(i32)]
#[derive(Copy, Clone)]
pub enum HandleType {
    Short = 0,
    ShortRecurrsion = 1,
    Strong = 2,
    Pinned = 3,
    Dependent = 6,
}

impl Default for HandleType {
    fn default() -> Self {
        Self::Short
    }
}
