//! `unsafe`-free, allocation-free, thread-safe, [`Clone`]able, `#[no_std]`-
//! compatible lifetime extension using an [`AtomicU32`] reference counter.
//!
//! This module provides the [`Atomic`] [`Binding`] implementation, which uses
//! an [`AtomicU32`] as a reference counter to track the number of active
//! [`Lich<T>`] clones. It does not require heap allocation, but it does require
//! the user to provide a mutable reference to a `u32` to use as the
//! reference counter.

use crate::{Pointer, shroud::Shroud};
use atomic_wait::{wait, wake_one};
use core::{
    mem::{ManuallyDrop, forget},
    ops::Deref,
    ptr::{self, NonNull, read},
    sync::atomic::{AtomicU32, Ordering},
};

pub struct Lich<T: ?Sized>(NonNull<T>, NonNull<AtomicU32>);
pub struct Soul<'a, P: ?Sized + 'a>(&'a AtomicU32, P);

unsafe impl<T: ?Sized> Send for Lich<T> where for<'a> &'a T: Send {}
unsafe impl<T: ?Sized> Sync for Lich<T> where for<'a> &'a T: Sync {}

impl<T: ?Sized> Lich<T> {
    pub fn bindings(&self) -> usize {
        bindings(unsafe { self.1.as_ref() })
    }
}

impl<T: ?Sized> Clone for Lich<T> {
    fn clone(&self) -> Self {
        let atomic = unsafe { self.1.as_ref() };
        atomic.fetch_add(1, Ordering::Relaxed);
        Self(self.0, self.1)
    }
}

impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        let atomic = unsafe { self.1.as_ref() };
        if atomic.fetch_sub(1, Ordering::Release) == 1 {
            // The soul might be waiting for this last lich to be dropped. Wake it up.
            wake_one(atomic);
        }
    }
}

impl<T: ?Sized> Deref for Lich<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T: ?Sized> AsRef<T> for Lich<T> {
    fn as_ref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<P: ?Sized> Soul<'_, P> {
    pub fn is_bound(&self) -> bool {
        self.bindings() > 0
    }

    pub fn bindings(&self) -> usize {
        bindings(self.0)
    }
}

impl<'a, P: 'a> Soul<'a, P> {
    pub const fn new(pointer: P, location: &'a mut u32) -> Self {
        Self(unsafe { AtomicU32::from_ptr(location) }, pointer)
    }

    pub fn sever(self) -> P {
        sever::<true>(self.0);
        unsafe { read(&ManuallyDrop::new(self).1) }
    }

    pub fn try_sever(self) -> Result<P, Self> {
        match sever::<false>(self.0) {
            Some(_) => Ok(unsafe { read(&ManuallyDrop::new(self).1) }),
            None => Err(self),
        }
    }
}

impl<P: Pointer + ?Sized> Soul<'_, P> {
    pub fn bind<T: Shroud<P::Target> + ?Sized>(&self) -> Lich<T> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Lich(T::shroud(self.1.pointer()), unsafe {
            NonNull::new_unchecked(self.0 as *const _ as *mut _)
        })
    }

    pub fn redeem<T: ?Sized>(&self, lich: Lich<T>) -> Result<bool, Lich<T>> {
        if ptr::addr_eq(self.0, lich.1.as_ptr()) {
            forget(lich);
            let bindings = self.0.fetch_sub(1, Ordering::Relaxed);
            Ok(bindings == 0)
        } else {
            Err(lich)
        }
    }
}

impl<P: ?Sized> Deref for Soul<'_, P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<P: ?Sized> AsRef<P> for Soul<'_, P> {
    fn as_ref(&self) -> &P {
        &self.1
    }
}

impl<P: ?Sized> Drop for Soul<'_, P> {
    fn drop(&mut self) {
        sever::<true>(self.0);
    }
}

fn sever<const WAIT: bool>(count: &AtomicU32) -> Option<bool> {
    loop {
        match count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
            Ok(0) => break Some(true),
            Ok(u32::MAX) | Err(u32::MAX) => break Some(false),
            Ok(value) | Err(value) if WAIT => wait(count, value),
            Ok(_) | Err(_) => break None,
        }
    }
}

fn bindings(count: &AtomicU32) -> usize {
    let count = count.load(Ordering::Relaxed);
    if count == u32::MAX { 0 } else { count as _ }
}
