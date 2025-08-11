#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "cell")]
pub mod cell;
#[cfg(feature = "lock")]
pub mod lock;
pub mod raw;
pub mod shroud;

use crate::shroud::Shroud;
use core::{
    mem::ManuallyDrop,
    ops::Deref,
    ptr::{NonNull, drop_in_place},
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
pub type RedeemResult<'a, T, B> = Result<Option<Soul<'a, B>>, (Lich<T, B>, Soul<'a, B>)>;

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

impl<T: ?Sized, B: Bind<Data<T>: Default> + ?Sized> Default for Lich<T, B> {
    fn default() -> Self {
        Self(B::Data::default())
    }
}

impl<T: ?Sized, B: Bind + ?Sized> Drop for Lich<T, B> {
    fn drop(&mut self) {
        self.0.try_sever();
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
        // # Safety
        // The `Option<NonNull<T>>` can only be `Some` as per the check in
        // `Lich<T>::borrow` and could not have been swapped for `None` since it is
        // protected by its corresponding `B::Refer` guard.
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

fn redeem<'a, T: ?Sized + 'a, B: Bind + ?Sized>(
    lich: Lich<T, B>,
    soul: Soul<'a, B>,
    bound: bool,
) -> RedeemResult<'a, T, B> {
    if B::are_bound(&lich.0, &soul.0) {
        let mut lich = ManuallyDrop::new(lich);
        unsafe { drop_in_place(&mut lich.0) };
        if bound && B::is_life_bound(&soul.0) {
            Ok(Some(soul))
        } else {
            let mut soul = ManuallyDrop::new(soul);
            unsafe { drop_in_place(&mut soul.0) };
            Ok(None)
        }
    } else {
        Err((lich, soul))
    }
}

#[allow(dead_code)]
mod fail {
    macro_rules! fail {
        ($function: ident, $block: block) => {
            #[doc = concat!("```compile_fail\n", stringify!($block), "\n```")]
            const fn $function() {}
        };
    }

    fail!(can_not_mutate_while_soul_lives, {
        use phylactery::raw::ritual;

        let mut value = 'a';
        let mut function = |letter| value = letter;
        let (lich, soul) = ritual::<_, dyn FnMut(char)>(&function);
        function('b');
    });

    fail!(can_not_drop_while_soul_lives, {
        use phylactery::raw::ritual;

        let mut value = 'a';
        let mut function = |letter| value = letter;
        let (lich, soul) = ritual::<_, dyn FnMut(char)>(&function);
        drop(function);
    });

    fail!(can_not_clone_lich, {
        use phylactery::raw::ritual;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn()>(&function);
        lich.clone();
    });

    fail!(can_not_clone_soul, {
        use phylactery::raw::ritual;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn()>(&function);
        soul.clone();
    });

    fail!(can_not_send_cell_to_thread, {
        use phylactery::cell::ritual;
        use std::thread::spawn;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn() + Send + Sync>(&function);
        spawn(move || lich);
    });

    fail!(can_not_send_lock_unsync_to_thread, {
        use phylactery::lock::ritual;
        use std::thread::spawn;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn() + Send>(&function);
        spawn(move || lich);
    });

    fail!(can_not_send_raw_unsync_to_thread, {
        use phylactery::raw::ritual;
        use std::thread::spawn;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn() + Send>(&function);
        spawn(move || lich);
    });

    fail!(can_not_create_default_lich, {
        use phylactery::raw::Lich;
        Lich::<()>::default();
    });
}
