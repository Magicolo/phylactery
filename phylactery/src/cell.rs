use crate::{Pointer, UniquePointer, shroud::Shroud};
use core::{
    borrow::Borrow,
    cell::Cell,
    mem::{ManuallyDrop, forget},
    ops::Deref,
    ptr::{NonNull, drop_in_place, read},
};

pub struct Lich<T: ?Sized> {
    count: NonNull<Cell<u32>>,
    data: NonNull<T>,
}

pub struct Soul<P, C: UniquePointer<Target = u32>> {
    data: P,
    count: C,
}

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
        if sever::<false>(self.count_ref()) {
            Ok(self.consume())
        } else {
            Err(self)
        }
    }

    pub fn bind<T: Shroud<P::Target> + ?Sized>(&self) -> Lich<T> {
        increment(self.count_ref());
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
    pub fn bindings(&self) -> usize {
        bindings(self.count_ref()) as _
    }

    pub fn redeem<T: ?Sized>(&self, lich: Lich<T>) -> Result<usize, Lich<T>> {
        if self.count_ptr() == lich.count {
            forget(lich);
            let bindings = decrement(self.count_ref());
            Ok(bindings as _)
        } else {
            Err(lich)
        }
    }

    fn count_ref(&self) -> &Cell<u32> {
        unsafe { self.count_ptr().as_ref() }
    }

    fn count_ptr(&self) -> NonNull<Cell<u32>> {
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

    fn count_ref(&self) -> &Cell<u32> {
        unsafe { self.count.as_ref() }
    }

    fn data_ref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> Clone for Lich<T> {
    fn clone(&self) -> Self {
        increment(self.count_ref());
        Self {
            data: self.data,
            count: self.count,
        }
    }
}

impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        decrement(self.count_ref());
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

fn sever<const PANIC: bool>(count: &Cell<u32>) -> bool {
    match count.get() {
        0 => {
            count.set(u32::MAX);
            true
        }
        value if PANIC => panic!("{value} `Lich<T>`es have not been redeemed"),
        _ => false,
    }
}

fn bindings(count: &Cell<u32>) -> u32 {
    let count = count.get();
    if count == u32::MAX { 0 } else { count }
}

fn increment(count: &Cell<u32>) -> u32 {
    let value = count.get();
    assert!(value < u32::MAX - 1);
    count.set(value + 1);
    value
}

fn decrement(count: &Cell<u32>) -> u32 {
    let value = count.get();
    debug_assert!(value > 0);
    count.set(value - 1);
    value
}
