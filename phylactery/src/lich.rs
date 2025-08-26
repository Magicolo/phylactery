use crate::Bind;
use core::{borrow::Borrow, marker::PhantomData, ops::Deref, ptr::NonNull};

pub struct Lich<T: ?Sized, B: Bind + ?Sized> {
    pub(crate) _marker: PhantomData<B>,
    pub(crate) data: NonNull<T>,
    pub(crate) count: NonNull<u32>,
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

    fn bind_ref(&self) -> &B {
        unsafe { B::shroud(self.count).as_ref() }
    }

    fn data_ref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Clone for Lich<T, B> {
    fn clone(&self) -> Self {
        self.bind_ref().increment();
        Self {
            data: self.data,
            count: self.count,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Drop for Lich<T, B> {
    fn drop(&mut self) {
        match self.bind_ref().decrement() {
            0 | u32::MAX => unreachable!(),
            1 => self.bind_ref().redeem(),
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
