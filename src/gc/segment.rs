use bitvec::{BitArr, order::Lsb0};

use crate::objects::*;
use crate::utils::IndexOfPtr;

pub struct Segment {
    data: [usize; Self::POINTER_SIZE],
    mark: BitArr!(for Segment::POINTER_SIZE, in usize, Lsb0),
    pin: BitArr!(for Segment::POINTER_SIZE, in usize, Lsb0),
    finalization_pending: BitArr!(for Segment::POINTER_SIZE, in usize, Lsb0),
    alloc_completed: bool,
    alive_bytes: usize,
    available_from: usize,
}

pub trait Seg {
    fn data(&self) -> &[usize];
    fn iter(&self) -> Box<dyn Iterator<Item = ObjectRef> + 'static>;
    fn contains(&self, or: ObjectRef) -> bool;
    fn find_object(&self, or_maybe: ObjectRef) -> Option<ObjectRef>;
    fn mark_object(&mut self, or: ObjectRef, pin: bool) -> Result<bool, ()>;
    fn is_marked(&self, or: ObjectRef) -> Result<bool, ()>;
    fn is_pinned(&self, or: ObjectRef) -> Result<bool, ()>;
    fn contains_pinned(&self) -> bool;
    fn clear_flags(&mut self);
    fn set_finalization_pending(&mut self, or: ObjectRef, pending: bool) -> Result<(), ()>;
    fn get_finalization_pending(&self, or: ObjectRef) -> Result<bool, ()>;
    fn sweep(&mut self) -> bool;
    fn set_alloc_completed(&mut self);
    fn get_alloc_completed(&self) -> bool;
    fn alive_bytes(&self) -> usize;
    fn available_space_with_header(&mut self) -> &mut [usize];
}

impl Segment {
    pub const SIZE: usize = 32768;
    pub const POINTER_SIZE: usize = Self::SIZE / size_of::<usize>();

    pub fn new_boxed() -> Box<Self> {
        unsafe { Box::new_zeroed().assume_init() }
    }

    fn get_index(&self, or: ObjectRef) -> Result<usize, ()> {
        self.data.index_of(or as *mut usize).ok_or(())
    }

    fn iter_raw(&self) -> RawSegmentIter {
        RawSegmentIter { range: self.data.as_ptr_range(), next: &raw const self.data[1] }
    }
}

impl Seg for Segment {
    fn data(&self) -> &[usize] {
        &self.data
    }

    fn iter(&self) -> Box<dyn Iterator<Item = ObjectRef> + 'static> {
        unsafe {
            Box::new(
                self.iter_raw()
                    .filter_map(|(or, _)| ((*or).method_table != FREE_MT).then_some(or)))
        }
    }

    fn contains(&self, or: ObjectRef) -> bool {
        self.data.as_ptr_range().contains(&(or as *const usize))
    }

    fn find_object(&self, or_maybe: ObjectRef) -> Option<ObjectRef> {
        unsafe {
            self.iter_raw().find_map(|(o, size)|
                ((*o).method_table != FREE_MT && (or_maybe.byte_offset_from(o) as usize) < size)
                    .then_some(o))
        }
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

    fn contains_pinned(&self) -> bool {
        self.pin.any()
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
        let mut alive_bytes = 0;
        let mut empty_from: Option<ObjectRef> = None;

        fn mark_as_empty(from: ObjectRef, to: ObjectRef) {
            unsafe {
                (*from) = Object {
                    method_table: FREE_MT,
                    component_count: (to.byte_offset_from_unsigned(from) - Object::BASE_SIZE) as u32,
                }
            }
        }

        let mut iter = self.iter_raw();
        for (or, size) in iter.by_ref() {
            if self.is_marked(or).unwrap() {
                alive_bytes += size;
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

        let end = empty_from.unwrap_or(iter.next as ObjectRef);
        self.available_from = self.get_index(end).map_or(Self::POINTER_SIZE, |i| i - 1);
        self.alive_bytes = alive_bytes;
        alive_bytes != 0
    }

    fn set_alloc_completed(&mut self) {
        self.alloc_completed = true;
    }

    fn get_alloc_completed(&self) -> bool {
        self.alloc_completed
    }

    fn alive_bytes(&self) -> usize {
        self.alive_bytes
    }

    fn available_space_with_header(&mut self) -> &mut [usize] {
        &mut self.data[self.available_from..]
    }
}

struct RawSegmentIter {
    pub next: *const usize,
    range: std::ops::Range<*const usize>,
}

impl Iterator for RawSegmentIter {
    type Item = (ObjectRef, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.range.contains(&self.next) {
            return None;
        }

        let obj = unsafe { &*(self.next as ObjectRef) };
        if obj.method_table.is_null() {
            return None;
        }

        let prev = self.next as ObjectRef;
        let size = obj.total_size_aligned();
        self.next = self.next.wrapping_byte_add(size);
        return Some((prev, size));
    }
}

pub struct LargeSegment {
    data: Box<[usize]>,
    mark: bool,
    finalization_pending: bool,
}

impl LargeSegment {
    pub fn new(size: usize) -> Self {
        debug_assert!(size % size_of::<usize>() == 0);
        Self {
            data: Box::from_iter(vec![0; size / size_of::<usize>() + 1]),
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

    fn contains_pinned(&self) -> bool {
        true
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

    fn sweep(&mut self) -> bool { self.mark }
    
    fn set_alloc_completed(&mut self) {}

    fn get_alloc_completed(&self) -> bool { true }

    fn alive_bytes(&self) -> usize { self.data.len() * size_of::<usize>() }

    fn available_space_with_header(&mut self) -> &mut [usize] { &mut [] }
}
