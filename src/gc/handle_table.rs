use bitvec::{BitArr, order::Lsb0};

use crate::objects::{GcHandle, ObjectHandle};
use crate::utils::IndexOfPtr;

pub struct HandleTable {
    segments: Vec<Box<HandleTableSegment>>,
}

pub struct HandleTableSegment {
    pub handles: [GcHandle; HandleTable::SEGMENT_SIZE],
    pub used_map: BitArr!(for HandleTable::SEGMENT_SIZE, in usize, Lsb0),
    pub used_count: usize,
}

impl HandleTable {
    const SEGMENT_SIZE: usize = 1000;

    pub fn new() -> Self {
        Self { segments: vec![] }
    }
    
    pub fn iter(&self) -> impl Iterator<Item = &GcHandle> {
        self.segments.iter().flat_map(|s|
            s.handles.iter().enumerate().filter_map(|(idx, h)| (s.used_map[idx] as bool).then_some(h)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut GcHandle> {
        self.segments.iter_mut().flat_map(|s|
            s.handles.iter_mut().enumerate().filter_map(|(idx, h)| (s.used_map[idx] as bool).then_some(h)))
    }

    pub fn create_new(&mut self) -> ObjectHandle {
        for seg in self.segments.iter_mut().filter(|s| s.used_count != Self::SEGMENT_SIZE) {
            for i in 0..Self::SEGMENT_SIZE {
                if !seg.used_map.replace(i, true) {
                    seg.used_count += 1;
                    return &raw mut seg.handles[i];
                }
            }
        }

        let mut seg = unsafe { Box::<HandleTableSegment>::new_zeroed().assume_init() };
        seg.used_count = 1;
        seg.used_map.set(0, true);
        let ptr = &raw mut seg.handles[0];
        self.segments.push(seg);
        ptr
    }

    pub fn contains(&self, h: ObjectHandle) -> bool {
        self.segments.iter().any(|s| s.handles.index_of(h).is_some_and(|idx| s.used_map[idx]))
    }

    pub fn remove(&mut self, h: ObjectHandle) -> Result<(), ()> {
        let seg = self.segments.iter_mut().find(|s| s.handles.as_ptr_range().contains(&(h as *const GcHandle))).ok_or(())?;
        if seg.used_map.replace(seg.handles.index_of(h).unwrap(), false) {
            seg.used_count -= 1;
        }
        unsafe { *h = GcHandle::default(); }
        Ok(())
    }
}
