use bitvec::{BitArr, order::Lsb0};

use crate::objects::*;
use crate::utils::IndexOfPtr;

pub struct Segment {
    data: [usize; Self::FLAGS_SIZE],
    mark: BitArr!(for Segment::FLAGS_SIZE, in usize, Lsb0),
    pin: BitArr!(for Segment::FLAGS_SIZE, in usize, Lsb0),
    finalization_pending: BitArr!(for Segment::FLAGS_SIZE, in usize, Lsb0),
    alloc_completed: bool,
}

pub trait Seg {
    fn data(&self) -> &[usize];
    fn iter(&self) -> Box<dyn Iterator<Item = ObjectRef> + 'static>;
    fn contains(&self, or: ObjectRef) -> bool;
    fn find_object(&self, or_maybe: ObjectRef) -> Option<ObjectRef>;
    fn mark_object(&mut self, or: ObjectRef, pin: bool) -> Result<bool, ()>;
    fn is_marked(&self, or: ObjectRef) -> Result<bool, ()>;
    fn is_pinned(&self, or: ObjectRef) -> Result<bool, ()>;
    fn clear_flags(&mut self);
    fn set_finalization_pending(&mut self, or: ObjectRef, pending: bool) -> Result<(), ()>;
    fn get_finalization_pending(&self, or: ObjectRef) -> Result<bool, ()>;
    fn sweep(&mut self) -> bool;
    fn set_alloc_completed(&mut self);
    fn get_alloc_completed(&self) -> bool;
}

impl Segment {
    pub const SIZE: usize = 32768;
    pub const FLAGS_SIZE: usize = Self::SIZE / size_of::<usize>();

    pub fn new_boxed() -> Box<Self> {
        unsafe { Box::new_zeroed().assume_init() }
    }

    fn get_index(&self, or: ObjectRef) -> Result<usize, ()> {
        self.data.index_of(or as *mut usize).ok_or(())
    }

    fn iter_raw(&self) -> impl Iterator<Item = ObjectRef> + 'static {
        RawSegmentIter { range: self.data.as_ptr_range(), next: &raw const self.data[1] }
    }
}

impl Seg for Segment {
    fn data(&self) -> &[usize] {
        &self.data
    }

    fn iter(&self) -> Box<dyn Iterator<Item = ObjectRef> + 'static> {
        Box::new(self.iter_raw().filter(|or| unsafe { (**or).method_table != &Object::EMPTY }))
    }

    fn contains(&self, or: ObjectRef) -> bool {
        self.data.as_ptr_range().contains(&(or as *const usize))
    }

    fn find_object(&self, or_maybe: ObjectRef) -> Option<ObjectRef> {
        self.iter_raw().find(|o| unsafe { (**o).method_table != &Object::EMPTY && or_maybe.byte_offset_from_unsigned(*o) < (**o).total_size_aligned() })
    }

    fn mark_object(&mut self, or: ObjectRef, pin: bool) -> Result<bool, ()> {
        let index = self.get_index(or)?;
        if pin {
            self.pin.set(index, true);
        }
        Ok(!self.mark.replace(index, true))
    }

    fn is_marked(&self, or: ObjectRef) -> Result<bool, ()> {
        let index = self.get_index(or)?;
        Ok(self.mark[index])
    }

    fn is_pinned(&self, or: ObjectRef) -> Result<bool, ()> {
        let index = self.get_index(or)?;
        Ok(self.pin[index])
    }

    fn clear_flags(&mut self) {
        self.mark.fill(false);
        self.pin.fill(false);
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
        let mut empty_from: Option<ObjectRef> = None;

        fn mark_as_empty(from: ObjectRef, to: ObjectRef) {
            unsafe {
                (*from) = Object {
                    method_table: &Object::EMPTY,
                    component_count: ((to.byte_offset_from_unsigned(from) - Object::BASE_SIZE) / size_of::<usize>()) as u32,
                }
            }
        }

        for or in self.iter_raw() {
            if self.is_marked(or).unwrap() {
                alive = true;
                if let Some(last) = empty_from {
                    mark_as_empty(last, or);
                }
                empty_from = None;
            } else {
                empty_from = empty_from.or(Some(or));
            }
        }

        if let Some(last) = empty_from {
            unsafe { (*last).method_table = std::ptr::null() };
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

struct RawSegmentIter {
    next: *const usize,
    range: std::ops::Range<*const usize>,
}

impl Iterator for RawSegmentIter {
    type Item = ObjectRef;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.range.contains(&self.next) {
            return None;
        }

        let obj = unsafe { &*(self.next as ObjectRef) };
        if obj.method_table.is_null() {
            return None;
        }

        let prev = self.next as ObjectRef;
        self.next = self.next.wrapping_byte_add(obj.total_size_aligned());
        return Some(prev);
    }
}

pub struct LargeSegment {
    data: Box<[usize]>,
    mark: bool,
    finalization_pending: bool,
}

impl LargeSegment {
    pub fn new(size: usize) -> Self {
        assert!(size % size_of::<usize>() == 0);
        Self {
            data: Box::from_iter(vec![0; size / size_of::<usize>()]),
            mark: false,
            finalization_pending: false,
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

    fn iter(&self) -> Box<dyn Iterator<Item = ObjectRef> + 'static> {
        Box::new(std::iter::once(self.as_object_ref()))
    }

    fn contains(&self, or: ObjectRef) -> bool {
        self.data[1..].as_ptr_range().contains(&(or as *const usize))
    }

    fn find_object(&self, or_maybe: ObjectRef) -> Option<ObjectRef> {
        self.contains(or_maybe).then(|| self.as_object_ref())
    }

    fn mark_object(&mut self, or: ObjectRef, _: bool) -> Result<bool, ()> {
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

    fn is_pinned(&self, or: ObjectRef) -> Result<bool, ()> {
        if or == self.as_object_ref() {
            Ok(true)
        } else {
            Err(())
        }
    }

    fn clear_flags(&mut self) {
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

    fn sweep(&mut self) -> bool { !self.mark }
    
    fn set_alloc_completed(&mut self) {}

    fn get_alloc_completed(&self) -> bool { true }
}
