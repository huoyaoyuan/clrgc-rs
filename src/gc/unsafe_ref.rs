use std::cell::UnsafeCell;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};

pub struct UnsafeRef<T> {
    inner: Arc<UnsafeCell<T>>
}

impl<T> UnsafeRef<T> {
    pub fn new(value: T) -> Self {
        Self { inner: Arc::new(UnsafeCell::new(value)) }
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}

impl<T> Clone for UnsafeRef<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T> Deref for UnsafeRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.get() }
    }
}

impl<T> DerefMut for UnsafeRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
