use crate::{Pointer, UniquePointer, shroud::Shroud};
use atomic_wait::{wait, wake_one};
use core::{
    borrow::Borrow,
    mem::{ManuallyDrop, forget},
    ops::Deref,
    ptr::{NonNull, drop_in_place, read},
    sync::atomic::{AtomicU32, Ordering},
};

pub struct Lich<T: ?Sized> {
    count: NonNull<AtomicU32>,
    data: NonNull<T>,
}

pub struct Soul<P, C: UniquePointer<Target = u32>> {
    data: P,
    count: C,
}

unsafe impl<T: ?Sized> Send for Lich<T> where for<'a> &'a T: Send {}
unsafe impl<T: ?Sized> Sync for Lich<T> where for<'a> &'a T: Sync {}

#[cfg(feature = "std")]
impl<P: Pointer> Soul<P, Box<u32>> {
    pub fn new(data: P) -> Self {
        Self::new_with(data, Box::new(0))
    }
}

impl<P: Pointer, C: UniquePointer<Target = u32>> Soul<P, C> {
    pub const fn new_with(data: P, count: C) -> Self {
        Self { data, count }
    }

    pub fn sever(self) -> P {
        sever::<true>(self.count_ref());
        self.consume()
    }

    pub fn try_sever(self) -> Result<P, Self> {
        match sever::<false>(self.count_ref()) {
            Some(_) => Ok(self.consume()),
            None => Err(self),
        }
    }

    pub fn bind<T: Shroud<P::Target> + ?Sized>(&self) -> Lich<T> {
        self.count_ref().fetch_add(1, Ordering::Relaxed);
        Lich {
            count: self.count_ptr(),
            data: T::shroud(self.data.pointer()),
        }
    }

    fn consume(self) -> P {
        let mut soul = ManuallyDrop::new(self);
        unsafe { drop_in_place(&mut soul.count) };
        unsafe { read(&soul.data) }
    }
}

impl<P, C: UniquePointer<Target = u32>> Soul<P, C> {
    /// This method will only give out a mutable reference to `P` if no
    /// bindings to this [`Soul`] remain.
    pub fn get_mut(&mut self) -> Option<&mut P> {
        if self.bindings() == 0 {
            Some(&mut self.data)
        } else {
            None
        }
    }

    pub fn bindings(&self) -> usize {
        bindings(self.count_ref()) as _
    }

    pub fn redeem<T: ?Sized>(&self, lich: Lich<T>) -> Result<usize, Lich<T>> {
        if self.count_ptr() == lich.count {
            forget(lich);
            let bindings = self.count_ref().fetch_sub(1, Ordering::Relaxed);
            Ok(bindings as _)
        } else {
            Err(lich)
        }
    }

    fn count_ref(&self) -> &AtomicU32 {
        unsafe { self.count_ptr().as_ref() }
    }

    fn count_ptr(&self) -> NonNull<AtomicU32> {
        self.count.pointer().cast()
    }
}

impl<P, C: UniquePointer<Target = u32>> Deref for Soul<P, C> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<P, C: UniquePointer<Target = u32>> AsRef<P> for Soul<P, C> {
    fn as_ref(&self) -> &P {
        &self.data
    }
}

impl<P, C: UniquePointer<Target = u32>> Borrow<P> for Soul<P, C> {
    fn borrow(&self) -> &P {
        &self.data
    }
}

impl<P, C: UniquePointer<Target = u32>> Drop for Soul<P, C> {
    fn drop(&mut self) {
        sever::<true>(self.count_ref());
    }
}

impl<T: ?Sized> Lich<T> {
    pub fn bindings(&self) -> usize {
        bindings(self.count_ref()) as _
    }

    fn count_ref(&self) -> &AtomicU32 {
        unsafe { self.count.as_ref() }
    }

    fn data_ref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> Clone for Lich<T> {
    fn clone(&self) -> Self {
        self.count_ref().fetch_add(1, Ordering::Relaxed);
        Self {
            data: self.data,
            count: self.count,
        }
    }
}

impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        match self.count_ref().fetch_sub(1, Ordering::Release) {
            0 | u32::MAX => unreachable!(),
            // The soul might be waiting for this last lich to be dropped. Wake it up.
            1 => wake_one(self.count.as_ptr()),
            _ => {}
        }
    }
}

impl<T: ?Sized> Borrow<T> for Lich<T> {
    fn borrow(&self) -> &T {
        self.data_ref()
    }
}

impl<T: ?Sized> Deref for Lich<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data_ref()
    }
}

impl<T: ?Sized> AsRef<T> for Lich<T> {
    fn as_ref(&self) -> &T {
        self.data_ref()
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

fn bindings(count: &AtomicU32) -> u32 {
    let count = count.load(Ordering::Relaxed);
    if count == u32::MAX { 0 } else { count }
}
