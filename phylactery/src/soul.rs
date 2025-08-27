use crate::{Binding, lich::Lich, shroud::Shroud};
use core::{
    borrow::Borrow,
    marker::PhantomPinned,
    mem::{ManuallyDrop, forget},
    ops::Deref,
    pin::Pin,
    ptr::{self, NonNull, drop_in_place, read},
};

#[derive(Debug)]
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

    #[cfg(feature = "std")]
    pub fn sever(self: Pin<Box<Self>>) -> Box<Self> {
        self.bind.sever::<true>();
        // Safety: all bindings have been severed, guaranteed by `B::sever`.
        unsafe { self.unpin() }
    }

    #[cfg(feature = "std")]
    pub fn try_sever(self: Pin<Box<Self>>) -> Result<Box<Self>, Pin<Box<Self>>> {
        if self.bind.sever::<false>() {
            // Safety: all bindings have been severed, guaranteed by `B::sever`.
            Ok(unsafe { self.unpin() })
        } else {
            Err(self)
        }
    }

    pub fn consume(self) -> T {
        // No need to run `<Soul as Drop>::drop` since no `Lich` can be bound, given by
        // this unpinned `Soul`.
        let mut soul = ManuallyDrop::new(self);
        unsafe { drop_in_place(&mut soul.bind) };
        unsafe { read(&soul.value) }
    }

    /// # Safety
    ///
    /// It **must** be the case the all bindings to [`Lich`]es have been severed
    /// before calling this function.
    #[cfg(feature = "std")]
    unsafe fn unpin(self: Pin<Box<Self>>) -> Box<Self> {
        debug_assert_eq!(self.bindings(), 0);
        // Safety: no `Lich`es are bound, the `Soul` can be unpinned.
        unsafe { Pin::into_inner_unchecked(self) }
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

    fn value_ptr(self: Pin<&Self>) -> NonNull<T> {
        // Safety: because `Soul` is pinned, it is safe to take pointers to it given
        // that those pointers are no longer accessible if the `Soul` is dropped which
        // is guaranteed by `B: Binding`
        unsafe { NonNull::new_unchecked(&self.value as *const _ as _) }
    }

    fn bind_ptr(self: Pin<&Self>) -> NonNull<B> {
        // Safety: because `Soul` is pinned, it is safe to take pointers to it given
        // that those pointers are no longer accessible if the `Soul` is dropped which
        // is guaranteed by `B: Binding`
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
