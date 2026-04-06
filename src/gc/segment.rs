use bitvec::{BitArr, order::Lsb0};

use crate::objects::{Object, ObjectRef};

pub struct Segment {
    data: [u8; Self::SIZE],
    mark: BitArr!(for Segment::FLAGS_SIZE, in u8, Lsb0),
}

pub trait Seg {
    fn data(&self) -> &[u8];
    fn for_each_obj(&self, callback: &mut dyn FnMut(ObjectRef));
    fn contains(&self, or: ObjectRef) -> bool;
    fn find_object(&self, or: ObjectRef) -> Option<ObjectRef>;
    fn mark_object(&mut self, or: ObjectRef) -> Result<bool, ()>;
    fn is_marked(&self, or: ObjectRef) -> Result<bool, ()>;
    fn clear_mark(&mut self);
    fn sweep(&mut self) -> bool;
}

impl Segment {
    pub const SIZE : usize = 32768;
    pub const FLAGS_SIZE : usize = Self::SIZE / size_of::<usize>();

    pub fn new_boxed() -> Box::<Self> {
        unsafe { Box::new_zeroed().assume_init() }
    }

    fn get_index(&self, or: ObjectRef) -> Result<usize, ()> {
        let range = self.data.as_ptr_range();
        let bptr = or as *const u8;
        if range.contains(&bptr) {
            unsafe { Ok(bptr.offset_from(range.start) as usize / size_of::<usize>()) }
        } else {
            Err(())
        }
    }
}

impl Seg for Segment {
    fn data(&self) -> &[u8] {
        &self.data
    }

    fn for_each_obj(&self, callback: &mut dyn FnMut(ObjectRef)) {
        let range = self.data.as_ptr_range();
        let mut ptr = &self.data[size_of::<usize>()] as *const u8;
        while range.contains(&ptr) {
            let obj = unsafe { &*(ptr as ObjectRef) };
            if obj.method_table.is_null() {
                return;
            }
            if obj.method_table != &Object::EMPTY {
                callback(ptr as ObjectRef);
            }
            ptr = ptr.wrapping_add(obj.total_size_aligned());
        }
    }

    fn contains(&self, or: ObjectRef) -> bool {
        self.data.as_ptr_range().contains(&(or as *const u8))
    }

    fn find_object(&self, or: ObjectRef) -> Option<ObjectRef> {
        let range = self.data.as_ptr_range();
        let mut ptr = &self.data[size_of::<usize>()] as *const u8;
        while range.contains(&ptr) {
            let obj = unsafe { &*(ptr as ObjectRef) };
            if obj.method_table.is_null() {
                return None;
            }
            let next_ptr = ptr.wrapping_add(obj.total_size_aligned());
            let bptr = or as *const u8;
            if ptr <= bptr && next_ptr > bptr {
                return Some(ptr as ObjectRef);
            }
            ptr = next_ptr;
        }

        None
    }

    fn mark_object(&mut self, or: ObjectRef) -> Result<bool, ()> {
        let index = self.get_index(or)?;
        Ok(!self.mark.replace(index, true))
    }

    fn is_marked(&self, or: ObjectRef) -> Result<bool, ()> {
        let index = self.get_index(or)?;
        Ok(self.mark[index])
    }

    fn clear_mark(&mut self) {
        self.mark.fill(false);
    }

    fn sweep(&mut self) -> bool {
        let mut alive = false;

        let range = self.data.as_ptr_range();
        let mut ptr = &self.data[size_of::<usize>()] as *const u8;
        let mut empty_from: Option<ObjectRef> = None;

        fn mark_as_empty(from: ObjectRef, to: ObjectRef) {
            unsafe {
                (*from) = Object {
                    method_table: &Object::EMPTY,
                    component_count: ((to.byte_offset_from_unsigned(from) - Object::BASE_SIZE) / size_of::<usize>()) as u32,
                }
            }
        }

        while range.contains(&ptr) {
            let or = ptr as ObjectRef;
            let obj = unsafe { &*or };
            if obj.method_table.is_null() {
                break;
            }

            if self.is_marked(or).unwrap() {
                alive = true;
                if let Some(last) = empty_from {
                    mark_as_empty(last, or);
                }
                empty_from = None;
            } else {
                empty_from = empty_from.or(Some(or));
            }

            ptr = ptr.wrapping_add(obj.total_size_aligned());
        }
        
        if let Some(last) = empty_from {
            mark_as_empty(last, ptr as ObjectRef);
        }

        alive
    }
}

pub struct LargeSegment {
    data: Box<[u8]>,
    alive: bool,
    mark: bool,
}

impl LargeSegment {
    pub fn new(size: usize) -> Self {
        Self { data: Box::from_iter(vec![0; size]), alive: true, mark: false }
    }

    fn as_object_ref(&self) -> ObjectRef {
        self.data[size_of::<usize>()] as ObjectRef
    }
}

impl Seg for LargeSegment {
    fn data(&self) -> &[u8] {
        self.data.as_ref()
    }

    fn for_each_obj(&self, callback: &mut dyn FnMut(ObjectRef)) {
        if self.alive {
            callback(self.as_object_ref());
        }
    }

    fn contains(&self, or: ObjectRef) -> bool {
        self.data.as_ptr_range().contains(&(or as *const u8))
    }

    fn find_object(&self, or: ObjectRef) -> Option<ObjectRef> {
        if self.contains(or) { Some(self.as_object_ref()) } else { None }
    }

    fn mark_object(&mut self, or: ObjectRef) -> Result<bool, ()> {
        if self.contains(or) {
            let old = self.mark;
            self.mark = true;
            Ok(!old)
        } else {
            Err(())
        }
    }

    fn is_marked(&self, or: ObjectRef) -> Result<bool, ()> {
        if self.contains(or) {
            Ok(self.mark)
        } else {
            Err(())
        }
    }

    fn clear_mark(&mut self) {
        self.mark = false;
    }

    fn sweep(&mut self) -> bool {
        self.alive = self.mark;
        self.mark
    }
}
