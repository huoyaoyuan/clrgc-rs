use crate::objects::ObjectRef;

pub struct Segment {
    pub size: usize,
    pub data: Box<[u8]>
}

fn align_to_ptr(size: u32) -> usize {
    let mask = size_of::<usize>() - 1;
    (size as usize + mask) & !mask
}

impl Segment {
    pub fn new(size: usize) -> Self {
        Self { size, data: Box::from_iter(vec![0; size]) }
    }

    pub fn for_each_obj<F>(&self, mut callback: F) where F : FnMut(ObjectRef) {
        let range = self.data.as_ptr_range();
        let mut ptr = &self.data[size_of::<usize>()] as *const u8;
        while range.contains(&ptr) {
            let obj = unsafe { &*(ptr as ObjectRef) };
            if obj.method_table.is_null() {
                return;
            }
            callback(ptr as ObjectRef);
            ptr = ptr.wrapping_add(align_to_ptr(obj.total_size()));
        }
    }

    // pub fn iter_objects(&self) -> impl Iterator<Item = ObjectRef> {
    //     let it = std::iter::iter!{|| {
    //         let range = self.data.as_ptr_range();
    //         let mut ptr = &self.data[size_of::<usize>()] as *const u8;
    //         while range.contains(&ptr) {
    //             let obj = unsafe { &*(ptr as ObjectRef) };
    //             if obj.method_table.is_null() {
    //                 return;
    //             }
    //             yield ptr as ObjectRef;
    //             ptr = ptr.wrapping_add(align_to_ptr(obj.total_size()));
    //         }
    //     } }();
    //     it
    // }

    pub fn contains(&self, or: ObjectRef) -> bool {
        self.data.as_ptr_range().contains(&(or as *const u8))
    }

    pub fn find_object(&self, or: ObjectRef) -> Option<ObjectRef> {
        let range = self.data.as_ptr_range();
        let mut ptr = &self.data[size_of::<usize>()] as *const u8;
        while range.contains(&ptr) {
            let obj = unsafe { &*(ptr as ObjectRef) };
            if obj.method_table.is_null() {
                return None;
            }
            let next_ptr = ptr.wrapping_add(align_to_ptr(obj.total_size()));
            let bptr = or as *const u8;
            if ptr <= bptr && next_ptr > bptr {
                return Some(ptr as ObjectRef);
            }
            ptr = next_ptr;
        }

        None
    }
}
