use crate::{Bind, Pointer, lich::Lich, shroud::Shroud};
use core::{
    borrow::Borrow,
    marker::PhantomData,
    mem::{ManuallyDrop, forget},
    ops::Deref,
    ptr::{NonNull, drop_in_place, read},
};

pub struct Soul<'a, P, B: Bind + ?Sized> {
    _marker: PhantomData<B>,
    data: P,
    count: Count<'a>,
}

pub enum Count<'a> {
    #[cfg(feature = "std")]
    Own(Box<u32>),
    Borrow(&'a mut u32),
}

impl Count<'_> {
    fn as_ptr(&self) -> NonNull<u32> {
        match self {
            #[cfg(feature = "std")]
            Count::Own(count) => unsafe { NonNull::new_unchecked(**count as *mut _) },
            Count::Borrow(count) => unsafe { NonNull::new_unchecked(*count as *const _ as *mut _) },
        }
    }
}

impl<'a> From<&'a mut u32> for Count<'a> {
    fn from(count: &'a mut u32) -> Self {
        Count::Borrow(count)
    }
}

#[cfg(feature = "std")]
impl<'a> From<Box<u32>> for Count<'a> {
    fn from(count: Box<u32>) -> Self {
        Count::Own(count)
    }
}

#[cfg(feature = "std")]
impl<P: Pointer, B: Bind> Soul<'static, P, B> {
    pub fn new(data: P) -> Self {
        Self::new_with(data, Count::Own(Box::new(0)))
    }
}

impl<'a, P: Pointer, B: Bind> Soul<'a, P, B> {
    pub fn new_with<C: Into<Count<'a>>>(data: P, count: C) -> Self {
        Self {
            data,
            count: count.into(),
            _marker: PhantomData,
        }
    }

    pub fn sever(self) -> P {
        self.get().sever::<true>();
        self.consume()
    }

    pub fn try_sever(self) -> Result<P, Self> {
        if self.get().sever::<false>() {
            Ok(self.consume())
        } else {
            Err(self)
        }
    }

    fn consume(self) -> P {
        let mut soul = ManuallyDrop::new(self);
        unsafe { drop_in_place(&mut soul.count) };
        unsafe { read(&soul.data) }
    }
}

impl<P: Pointer, B: Bind + ?Sized> Soul<'_, P, B> {
    pub fn bind<T: Shroud<P::Target> + ?Sized>(&self) -> Lich<T, B> {
        self.get().increment();
        Lich {
            count: self.count.as_ptr(),
            data: T::shroud(self.data.pointer()),
            _marker: PhantomData,
        }
    }
}

impl<P, B: Bind + ?Sized> Soul<'_, P, B> {
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
        self.get().bindings() as _
    }

    pub fn redeem<T: ?Sized>(&self, lich: Lich<T, B>) -> Result<usize, Lich<T, B>> {
        if self.count.as_ptr() == lich.count {
            forget(lich);
            let bindings = self.get().decrement();
            Ok(bindings as _)
        } else {
            Err(lich)
        }
    }

    fn get(&self) -> &B {
        unsafe { B::shroud(self.count.as_ptr()).as_ref() }
    }
}

impl<P, B: Bind + ?Sized> Deref for Soul<'_, P, B> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<P, B: Bind + ?Sized> AsRef<P> for Soul<'_, P, B> {
    fn as_ref(&self) -> &P {
        &self.data
    }
}

impl<P, B: Bind + ?Sized> Borrow<P> for Soul<'_, P, B> {
    fn borrow(&self) -> &P {
        &self.data
    }
}

impl<P, B: Bind + ?Sized> Drop for Soul<'_, P, B> {
    fn drop(&mut self) {
        self.get().sever::<true>();
    }
}
