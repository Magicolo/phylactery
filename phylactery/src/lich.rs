use crate::Bind;
use core::{borrow::Borrow, ops::Deref, ptr::NonNull};

pub struct Lich<T: ?Sized, B: Bind + ?Sized> {
    pub(crate) value: NonNull<T>,
    pub(crate) bind: NonNull<B>,
}

unsafe impl<T: ?Sized, B: Bind + ?Sized> Send for Lich<T, B>
where
    for<'a> &'a T: Send,
    for<'a> &'a B: Send,
{
}
unsafe impl<T: ?Sized, B: Bind + ?Sized> Sync for Lich<T, B>
where
    for<'a> &'a T: Sync,
    for<'a> &'a B: Sync,
{
}

impl<T: ?Sized, B: Bind + ?Sized> Lich<T, B> {
    pub fn bindings(&self) -> usize {
        self.bind_ref().bindings() as _
    }

    const fn bind_ref(&self) -> &B {
        unsafe { self.bind.as_ref() }
    }

    const fn data_ref(&self) -> &T {
        unsafe { self.value.as_ref() }
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Clone for Lich<T, B> {
    fn clone(&self) -> Self {
        self.bind_ref().increment();
        Self {
            value: self.value,
            bind: self.bind,
        }
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Drop for Lich<T, B> {
    fn drop(&mut self) {
        let bind = self.bind_ref();
        match bind.decrement() {
            0 | u32::MAX => unreachable!(),
            1 => bind.redeem(),
            _ => {}
        }
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Borrow<T> for Lich<T, B> {
    fn borrow(&self) -> &T {
        self.data_ref()
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Deref for Lich<T, B> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data_ref()
    }
}

impl<T: ?Sized, B: Bind + ?Sized> AsRef<T> for Lich<T, B> {
    fn as_ref(&self) -> &T {
        self.data_ref()
    }
}
