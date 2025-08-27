use crate::{Binding, lich::Lich, shroud::Shroud};
use core::{
    borrow::Borrow,
    marker::PhantomPinned,
    mem::{ManuallyDrop, forget},
    ops::Deref,
    pin::Pin,
    ptr::{self, NonNull, drop_in_place, read},
};

pub struct Soul<T: ?Sized, B: Binding> {
    _marker: PhantomPinned,
    bind: B,
    value: T,
}

impl<T, B: Binding> Soul<T, B> {
    pub const fn new(value: T) -> Self {
        Self {
            value,
            bind: B::NEW,
            _marker: PhantomPinned,
        }
    }

    pub fn pin(self) -> Pin<Box<Self>> {
        Box::pin(self)
    }

    pub fn sever(self: Pin<Box<Self>>) -> T {
        self.bind.sever::<true>();
        self.consume()
    }

    pub fn try_sever(self: Pin<Box<Self>>) -> Result<T, Pin<Box<Self>>> {
        if self.bind.sever::<false>() {
            Ok(self.consume())
        } else {
            Err(self)
        }
    }

    fn consume(self: Pin<Box<Self>>) -> T {
        let soul = *unsafe { Pin::into_inner_unchecked(self) };
        let mut soul = ManuallyDrop::new(soul);
        unsafe { drop_in_place(&mut soul.bind) };
        unsafe { read(&soul.value) }
    }
}

impl<T: ?Sized, B: Binding> Soul<T, B> {
    pub fn bind<S: Shroud<T> + ?Sized>(self: Pin<&Self>) -> Lich<S, B> {
        self.bind.increment();
        Lich {
            bind: self.bind_ptr(),
            value: S::shroud(self.value_ptr()),
        }
    }

    pub fn bindings(&self) -> usize {
        self.bind.count() as _
    }

    pub fn redeem<S: ?Sized>(&self, lich: Lich<S, B>) -> Result<usize, Lich<S, B>> {
        if ptr::eq(&self.bind, lich.bind.as_ptr()) {
            forget(lich);
            Ok(self.bind.decrement() as _)
        } else {
            Err(lich)
        }
    }

    fn value_ptr(&self) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(&self.value as *const _ as _) }
    }

    fn bind_ptr(&self) -> NonNull<B> {
        unsafe { NonNull::new_unchecked(&self.bind as *const _ as _) }
    }
}

impl<T: ?Sized, B: Binding> Deref for Soul<T, B> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: ?Sized, B: Binding> AsRef<T> for Soul<T, B> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized, B: Binding> Borrow<T> for Soul<T, B> {
    fn borrow(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized, B: Binding> Drop for Soul<T, B> {
    fn drop(&mut self) {
        self.bind.sever::<true>();
    }
}
