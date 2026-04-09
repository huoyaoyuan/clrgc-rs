use bitvec::{BitArr, order::Lsb0};

use crate::objects::{Object, ObjectRef};

pub struct Segment {
    data: [usize; Self::FLAGS_SIZE],
    mark: BitArr!(for Segment::FLAGS_SIZE, in usize, Lsb0),
    finalization_pending: BitArr!(for Segment::FLAGS_SIZE, in usize, Lsb0),
    alloc_completed: bool,
}

pub trait Seg {
    fn data(&self) -> &[usize];
    fn for_each_obj(&self, callback: &mut dyn FnMut(ObjectRef));
    fn for_each_obj_mut(&mut self, callback: &mut dyn FnMut(&mut dyn Seg, ObjectRef));
    fn contains(&self, or: ObjectRef) -> bool;
    fn find_object(&self, or: ObjectRef) -> Option<ObjectRef>;
    fn mark_object(&mut self, or: ObjectRef) -> Result<bool, ()>;
    fn is_marked(&self, or: ObjectRef) -> Result<bool, ()>;
    fn clear_mark(&mut self);
    fn set_finalization_pending(&mut self, or: ObjectRef, pending: bool) -> Result<(), ()>;
    fn get_finalization_pending(&self, or: ObjectRef) -> Result<bool, ()>;
    fn sweep(&mut self) -> bool;
    fn set_alloc_completed(&mut self);
    fn get_alloc_completed(&self) -> bool;
}

impl Segment {
    pub const SIZE : usize = 32768;
    pub const FLAGS_SIZE : usize = Self::SIZE / size_of::<usize>();

    pub fn new_boxed() -> Box::<Self> {
        unsafe { Box::new_zeroed().assume_init() }
    }

    fn get_index(&self, or: ObjectRef) -> Result<usize, ()> {
        let range = self.data.as_ptr_range();
        let bptr = or as *const usize;
        if range.contains(&bptr) {
            unsafe { Ok(bptr.offset_from(range.start) as usize) }
        } else {
            Err(())
        }
    }
}

impl Seg for Segment {
    fn data(&self) -> &[usize] {
        &self.data
    }

    fn for_each_obj(&self, callback: &mut dyn FnMut(ObjectRef)) {
        let range = self.data.as_ptr_range();
        let mut ptr = &raw const self.data[1];
        while range.contains(&ptr) {
            let obj = unsafe { &*(ptr as ObjectRef) };
            if obj.method_table.is_null() {
                return;
            }
            if obj.method_table != &Object::EMPTY {
                callback(ptr as ObjectRef);
            }
            ptr = ptr.wrapping_byte_add(obj.total_size_aligned());
        }
    }

    fn for_each_obj_mut(&mut self, callback: &mut dyn FnMut(&mut dyn Seg, ObjectRef)) {
        let range = self.data.as_ptr_range();
        let mut ptr = &raw const self.data[1];
        while range.contains(&ptr) {
            let obj = unsafe { &*(ptr as ObjectRef) };
            if obj.method_table.is_null() {
                return;
            }
            if obj.method_table != &Object::EMPTY {
                callback(self, ptr as ObjectRef);
            }
            ptr = ptr.wrapping_byte_add(obj.total_size_aligned());
        }
    }

    fn contains(&self, or: ObjectRef) -> bool {
        self.data.as_ptr_range().contains(&(or as *const usize))
    }

    fn find_object(&self, or: ObjectRef) -> Option<ObjectRef> {
        let range = self.data.as_ptr_range();
        let bptr = or as *const usize;
        let mut ptr = &raw const self.data[1];
        while range.contains(&ptr) {
            let obj = unsafe { &*(ptr as ObjectRef) };
            if obj.method_table.is_null() {
                return None;
            }
            let next_ptr = ptr.wrapping_byte_add(obj.total_size_aligned());
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

    fn set_finalization_pending(&mut self, or: ObjectRef, pending: bool) -> Result<(), ()> {
        let index = self.get_index(or)?;
        self.finalization_pending.replace(index, pending);
        Ok(())
    }

    fn get_finalization_pending(&self, or: ObjectRef) -> Result<bool, ()> {
        let index = self.get_index(or)?;
        Ok(self.finalization_pending[index])
    }

    fn sweep(&mut self) -> bool {
        let mut alive = false;

        let range = self.data.as_ptr_range();
        let mut ptr = &raw const self.data[1];
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
            let obj = unsafe { &mut *or };
            if obj.method_table.is_null() {
                break;
            }

            if self.is_marked(or).unwrap() || obj.needs_finalization() {
                alive = true;
                if let Some(last) = empty_from {
                    mark_as_empty(last, or);
                }
                empty_from = None;
            } else {
                empty_from = empty_from.or(Some(or));
            }

            ptr = ptr.wrapping_byte_add(obj.total_size_aligned());
        }
        
        if let Some(last) = empty_from {
            mark_as_empty(last, ptr as ObjectRef);
        }

        alive
    }
    
    fn set_alloc_completed(&mut self) {
        self.alloc_completed = true;
    }
    
    fn get_alloc_completed(&self) -> bool {
        self.alloc_completed
    }
}

pub struct LargeSegment {
    data: Box<[usize]>,
    alive: bool,
    mark: bool,
    finalization_pending: bool
}

impl LargeSegment {
    pub fn new(size: usize) -> Self {
        assert!(size % size_of::<usize>() == 0);
        Self {
            data: Box::from_iter(vec![0; size / size_of::<usize>()]),
            alive: true,
            mark: false,
            finalization_pending: false
        }
    }

    fn as_object_ref(&self) -> ObjectRef {
        &raw const self.data[1] as ObjectRef
    }
}

impl Seg for LargeSegment {
    fn data(&self) -> &[usize] {
        self.data.as_ref()
    }

    fn for_each_obj(&self, callback: &mut dyn FnMut(ObjectRef)) {
        if self.alive {
            let or = self.as_object_ref();
            callback(or);
        }
    }

    fn for_each_obj_mut(&mut self, callback: &mut dyn FnMut(&mut dyn Seg, ObjectRef)) {
        if self.alive {
            let or = self.as_object_ref();
            callback(self, or);
        }
    }

    fn contains(&self, or: ObjectRef) -> bool {
        self.data.as_ptr_range().contains(&(or as *const usize))
    }

    fn find_object(&self, or: ObjectRef) -> Option<ObjectRef> {
        if self.contains(or) { Some(self.as_object_ref()) } else { None }
    }

    fn mark_object(&mut self, or: ObjectRef) -> Result<bool, ()> {
        if or == self.as_object_ref() {
            let old = self.mark;
            self.mark = true;
            Ok(!old)
        } else {
            Err(())
        }
    }

    fn is_marked(&self, or: ObjectRef) -> Result<bool, ()> {
        if or == self.as_object_ref() {
            Ok(self.mark)
        } else {
            Err(())
        }
    }

    fn clear_mark(&mut self) {
        self.mark = false;
    }

    fn set_finalization_pending(&mut self, or: ObjectRef, pending: bool) -> Result<(), ()> {
        if or == self.as_object_ref() {
            self.finalization_pending = pending;
            Ok(())
        } else {
            Err(())
        }
    }

    fn get_finalization_pending(&self, or: ObjectRef) -> Result<bool, ()> {
        if or == self.as_object_ref() {
            Ok(self.finalization_pending)
        } else {
            Err(())
        }
    }

    fn sweep(&mut self) -> bool {
        self.alive = unsafe { self.mark || (*self.as_object_ref()).needs_finalization() };
        self.alive
    }
    
    fn set_alloc_completed(&mut self) { }
    
    fn get_alloc_completed(&self) -> bool { true }
}
