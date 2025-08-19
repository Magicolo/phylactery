//! Zero-cost, `unsafe`, allocation-free, thread-safe, `#[no_std]`-compatible
//! lifetime extension.
//!
//! This module provides the [`Raw`] [`Binding`] implementation, which is the
//! most performant but also the most dangerous variant. It offers a zero-cost
//! abstraction, meaning it introduces no heap allocations or reference-counting
//! overhead. The [`Lich<T>`] and [`Soul<P>`] are simple new-type wrappers
//! around raw pointers.

use crate::{Pointer, shroud::Shroud};
use core::{
    mem::{ManuallyDrop, forget},
    ops::Deref,
    ptr::{self, NonNull, read},
};

pub struct Lich<T: ?Sized>(NonNull<T>);
pub struct Soul<P: ?Sized>(usize, P);

unsafe impl<T: ?Sized> Send for Lich<T> where for<'a> &'a T: Send {}
unsafe impl<T: ?Sized> Sync for Lich<T> where for<'a> &'a T: Sync {}

impl<T: ?Sized> Lich<T> {
    /// # Safety
    ///
    /// The caller must ensure that the corresponding [`Soul<P>`] is still
    /// alive and in scope. Dropping the [`Soul<P>`] while this borrow is
    /// active will invalidate the pointer, leading to a `use-after-free`
    /// vulnerability.
    ///
    /// This variant offers no runtime checks to prevent this. It is the
    /// caller's responsibility to uphold this safety contract.
    pub unsafe fn get(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        panic();
    }
}

impl<P> Soul<P> {
    pub const fn new(pointer: P) -> Self {
        Self(0, pointer)
    }

    pub fn try_sever(self) -> Result<P, Self> {
        if self.is_bound() {
            Err(self)
        } else {
            Ok(unsafe { self.sever() })
        }
    }

    /// # Safety
    ///
    /// The caller must ensure that this [`Soul<P>`] has not bindings left
    /// (`self.bindings() == 0`) by [`Self::redeem`]ing all [`Lich<T>`] that
    /// have been bound to it.
    unsafe fn sever(self) -> P {
        debug_assert_eq!(self.bindings(), 0);
        // Even with a call to `is_bound` with a `panic!`, this method is always
        // `unsafe` to call. It is not enough that this thread panics to ensure safety;
        // all threads that have access to a bound `Lich` would need to panic as well.
        unsafe { read(&ManuallyDrop::new(self).1) }
    }
}

impl<P: ?Sized> Soul<P> {
    /// This method will only give out a mutable reference to `P` if no bindings
    /// to this [`Soul<P>`] remain.
    pub const fn get_mut(this: &mut Self) -> Option<&mut P> {
        if this.is_bound() {
            None
        } else {
            Some(&mut this.1)
        }
    }

    pub const fn is_bound(&self) -> bool {
        self.0 > 0
    }

    pub const fn bindings(&self) -> usize {
        self.0
    }
}

impl<P: Pointer> Soul<P> {
    pub fn redeem<T: ?Sized>(&mut self, lich: Lich<T>) -> Result<bool, Lich<T>> {
        let Some(bindings) = self.0.checked_sub(1) else {
            return Err(lich);
        };
        if ptr::addr_eq(self.1.pointer(), lich.0.as_ptr()) {
            forget(lich);
            self.0 = bindings;
            Ok(bindings == 0)
        } else {
            Err(lich)
        }
    }
}

impl<P: Pointer + ?Sized> Soul<P> {
    pub fn bind<T: Shroud<P::Target> + ?Sized>(&mut self) -> Lich<T> {
        self.0 += 1;
        Lich(T::shroud(self.1.pointer()))
    }
}

impl<P: Default> Default for Soul<P> {
    fn default() -> Self {
        Self::new(P::default())
    }
}

impl<P: ?Sized> Deref for Soul<P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<P: ?Sized> AsRef<P> for Soul<P> {
    fn as_ref(&self) -> &P {
        &self.1
    }
}

impl<P: ?Sized> Drop for Soul<P> {
    fn drop(&mut self) {
        if self.is_bound() {
            panic();
        }
    }
}

fn panic() -> bool {
    #[cfg(feature = "std")]
    if std::thread::panicking() {
        return false;
    }

    #[cfg(not(feature = "std"))]
    {
        use core::sync::atomic::{AtomicBool, Ordering};

        static PANIC: AtomicBool = AtomicBool::new(false);
        if PANIC.swap(true, Ordering::Relaxed) {
            return false;
        }
    }

    panic!("a `Lich<T>` has not been redeemed")
}
