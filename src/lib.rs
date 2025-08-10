#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "cell")]
pub mod cell;
#[cfg(feature = "lock")]
pub mod lock;
pub mod raw;
pub mod shroud;

/*
TODO:
- Myth: The [`Lich<T>`] is a being that forfeited its [`Soul<'a>`] through a ritual in order to become undead.
- Rust: The [`Lich<T>`] is a `&'a T` that forfeited its lifetime through some unsafe code in order to become `'static`.

- fill cheats.rs with the 2 scenarios below
- 2 scenarios come to mind for usage of the `Lich`:
    - thread local scoped state
        - for example, a profiler stores some thread-local data that lives on the stack in a static variable such that
        the profiler object does not need to be carried around
        - similarly for a logger scope a log prefix or values that live on the stack
        - normally, one would need to somehow move the values from the stack to a thread-local storage, since there is
        a `'static` requirement and this would simply not be possible for non-static values
    - scoped `thread/task::spawn`
        - since the `Lich` can have any chosen lifetime, it can choose `'static` in order to cross a thread boundary
        using a regular `spawn`, thus bridging and/or expanding the capabilities of threading libraries

- Even when hidden within a data handle, it must be known that the handle may escape the closure.
- Should it be possible to `Clone` the `Lich`?
    - It can be trivially cloned because of its `Arc`.
    - It may be unclear that using `borrow_mut` takes a `write` lock; thus can dead lock.
- What about not supporting `borrow_mut`?
*/
use crate::shroud::Shroud;
use core::{
    any::type_name,
    fmt,
    mem::ManuallyDrop,
    ops::Deref,
    ptr::{self, NonNull},
};

pub trait Bind {
    type Data<T: ?Sized>: Sever;
    type Life<'a>: Sever;
    type Refer<'a, T: ?Sized + 'a>;

    /// Splits the provided reference into its data part `Self::Data<T>` and
    /// its lifetime part `Self::Life<'a>`, binding them together.
    fn bind<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(
        value: &'a T,
    ) -> (Self::Data<S>, Self::Life<'a>);
    /// Checks whether the `Self::Data<T>` and `Self::Life<'a>` have been
    /// bound together with the same `Self::bind` call.
    fn are_bound<T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'_>) -> bool;
    fn is_life_bound(life: &Self::Life<'_>) -> bool;
    fn is_data_bound<T: ?Sized>(data: &Self::Data<T>) -> bool;
}

pub struct Soul<'a, B: Bind + ?Sized>(pub(crate) B::Life<'a>);
pub struct Lich<T: ?Sized, B: Bind + ?Sized>(pub(crate) B::Data<T>);
pub struct Guard<'a, T: ?Sized + 'a, B: Bind + ?Sized>(pub(crate) B::Refer<'a, T>);
pub struct RedeemError<'a, T: ?Sized, B: Bind + ?Sized>(Lich<T, B>, Soul<'a, B>);
pub type RedeemResult<'a, T, B> = Result<Option<Soul<'a, B>>, RedeemError<'a, T, B>>;

pub trait Sever {
    fn sever(&mut self) -> bool;

    fn try_sever(&mut self) -> Option<bool> {
        Some(self.sever())
    }
}

impl<T> Sever for Option<T> {
    fn sever(&mut self) -> bool {
        self.take().is_some()
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Lich<T, B> {
    pub fn is_bound(&self) -> bool {
        B::is_data_bound(&self.0)
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Lich<T, B> {
    pub fn sever(mut self) -> bool {
        self.0.sever()
    }

    pub fn try_sever(mut self) -> Result<bool, Self> {
        self.0.try_sever().ok_or(self)
    }
}

impl<B: Bind + ?Sized> Soul<'_, B> {
    pub fn sever(mut self) -> bool {
        self.0.sever()
    }

    pub fn try_sever(mut self) -> Result<bool, Self> {
        self.0.try_sever().ok_or(self)
    }
}

impl<B: Bind + ?Sized> Soul<'_, B> {
    pub fn is_bound(&self) -> bool {
        B::is_life_bound(&self.0)
    }
}

impl<T: ?Sized, B: Bind<Data<T>: Clone> + ?Sized> Clone for Lich<T, B> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Drop for Lich<T, B> {
    fn drop(&mut self) {
        self.0.sever();
    }
}

impl<B: Bind + ?Sized> Drop for Soul<'_, B> {
    fn drop(&mut self) {
        self.0.sever();
    }
}

impl<'a, T: ?Sized, B: Bind<Refer<'a, T>: Deref<Target = Option<NonNull<T>>>> + ?Sized> Deref
    for Guard<'a, T, B>
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.deref().as_ref().unwrap_unchecked().as_ref() }
    }
}

impl<'a, T: ?Sized, B: Bind<Refer<'a, T>: AsRef<Option<NonNull<T>>>> + ?Sized> AsRef<T>
    for Guard<'a, T, B>
{
    fn as_ref(&self) -> &T {
        unsafe { self.0.as_ref().as_ref().unwrap_unchecked().as_ref() }
    }
}

fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a, B: Bind + ?Sized>(
    value: &'a T,
) -> (Lich<S, B>, Soul<'a, B>) {
    let (data, life) = B::bind(value);
    (Lich(data), Soul(life))
}

impl<'a, T: ?Sized + 'a, B: Bind + ?Sized> fmt::Debug for RedeemError<'a, T, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to redeem `Lich<{}, {}>` and `Soul<'a, {}>` pair",
            type_name::<T>(),
            type_name::<B>(),
            type_name::<B>(),
        )
    }
}

impl<'a, T: ?Sized + 'a, B: Bind + ?Sized> fmt::Display for RedeemError<'a, T, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[rustversion::since(1.81)]
impl<'a, T: ?Sized + 'a, B: Bind + ?Sized> core::error::Error for RedeemError<'a, T, B> {}
#[rustversion::before(1.81)]
#[cfg(feature = "std")]
impl<'a, T: ?Sized + 'a, B: Bind + ?Sized> std::error::Error for RedeemError<'a, T, B> {}

impl<'a, T: ?Sized + 'a, B: Bind + ?Sized> RedeemError<'a, T, B> {
    pub fn into_inner(self) -> (Lich<T, B>, Soul<'a, B>) {
        (self.0, self.1)
    }
}

unsafe fn redeem<'a, T: ?Sized + 'a, B: Bind + ?Sized>(
    lich: Lich<T, B>,
    soul: Soul<'a, B>,
) -> RedeemResult<'a, T, B> {
    if B::are_bound(&lich.0, &soul.0) {
        let lich = ManuallyDrop::new(lich);
        drop(unsafe { ptr::read(&lich.0) });
        if B::is_life_bound(&soul.0) {
            Ok(Some(soul))
        } else {
            let soul = ManuallyDrop::new(soul);
            unsafe { ptr::read(&soul.0) };
            Ok(None)
        }
    } else {
        Err(RedeemError(lich, soul))
    }
}
