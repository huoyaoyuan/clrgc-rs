pub trait IndexOfPtr<T: Sized> {
    fn index_of(&self, ptr: *const T) -> Option<usize>;
}

impl<T: Sized> IndexOfPtr<T> for [T] {
    fn index_of(&self, ptr: *const T) -> Option<usize> {
        let diff = unsafe { ptr.offset_from(self.as_ptr()) as usize };
        (diff < self.len()).then_some(diff)
    }
}
