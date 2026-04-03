pub type ObjectHandle = usize;

#[repr(C)]
pub struct Object {
    method_table: usize,
    component_count: i32,
}

pub type ObjectRef = *mut Object;

#[repr(i32)]
#[derive(Clone)]
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
