use std::cell::UnsafeCell;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};

pub struct UnsafeRef<T: ?Sized> {
    inner: Arc<UnsafeCell<Box<T>>>
}

impl<T: ?Sized> UnsafeRef<T> {
    pub fn new(value: Box<T>) -> Self {
        Self { inner: Arc::new(UnsafeCell::new(value)) }
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}

impl<T: ?Sized> Clone for UnsafeRef<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: ?Sized> Deref for UnsafeRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.get() }
    }
}

impl<T: ?Sized> DerefMut for UnsafeRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
