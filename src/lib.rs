#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "atomic")]
pub mod atomic;
#[cfg(feature = "cell")]
pub mod cell;
#[cfg(feature = "lock")]
pub mod lock;
pub mod raw;
pub mod shroud;

use core::{mem::ManuallyDrop, ptr::drop_in_place};

pub trait Binding {
    type Data<T: ?Sized>: TrySever;
    type Life<'a>: Sever;

    /// Checks whether the `Self::Data<T>` and `Self::Life<'a>` have been
    /// bound together with the same `Self::bind` call.
    fn are_bound<T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'_>) -> bool;
    fn is_life_bound(life: &Self::Life<'_>) -> bool;
    fn is_data_bound<T: ?Sized>(data: &Self::Data<T>) -> bool;
}

pub struct Soul<'a, B: Binding + ?Sized>(pub(crate) B::Life<'a>);
pub struct Lich<T: ?Sized, B: Binding + ?Sized>(pub(crate) B::Data<T>);
pub type Pair<'a, T, B> = (Lich<T, B>, Soul<'a, B>);

pub trait Sever {
    fn sever(&mut self) -> bool;
}

pub trait TrySever {
    fn try_sever(&mut self) -> Option<bool>;
}

unsafe impl<T: ?Sized, B: Binding<Data<T>: Send> + ?Sized> Send for Lich<T, B> {}
unsafe impl<T: ?Sized, B: Binding<Data<T>: Sync> + ?Sized> Sync for Lich<T, B> {}
unsafe impl<'a, B: Binding<Life<'a>: Send> + ?Sized> Send for Soul<'a, B> {}
unsafe impl<'a, B: Binding<Life<'a>: Sync> + ?Sized> Sync for Soul<'a, B> {}

impl<T> Sever for Option<T> {
    fn sever(&mut self) -> bool {
        self.take().is_some()
    }
}

impl<T> TrySever for Option<T> {
    fn try_sever(&mut self) -> Option<bool> {
        Some(self.sever())
    }
}

impl<T: ?Sized, B: Binding + ?Sized> Lich<T, B> {
    pub fn is_bound(&self) -> bool {
        B::is_data_bound(&self.0)
    }
}

impl<T: ?Sized, B: Binding + ?Sized> Lich<T, B> {
    pub fn try_sever(mut self) -> Result<bool, Self> {
        self.0.try_sever().ok_or(self)
    }
}

impl<T: ?Sized, B: Binding<Data<T>: Sever> + ?Sized> Lich<T, B> {
    pub fn sever(mut self) -> bool {
        self.0.sever()
    }
}

impl<B: Binding + ?Sized> Soul<'_, B> {
    pub fn sever(mut self) -> bool {
        self.0.sever()
    }
}

impl<'a, B: Binding<Life<'a>: TrySever> + ?Sized> Soul<'a, B> {
    pub fn try_sever(mut self) -> Result<bool, Self> {
        self.0.try_sever().ok_or(self)
    }
}

impl<B: Binding + ?Sized> Soul<'_, B> {
    pub fn is_bound(&self) -> bool {
        B::is_life_bound(&self.0)
    }
}

impl<T: ?Sized, B: Binding<Data<T>: Clone> + ?Sized> Clone for Lich<T, B> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized, B: Binding<Data<T>: Default> + ?Sized> Default for Lich<T, B> {
    fn default() -> Self {
        Self(B::Data::default())
    }
}

impl<T: ?Sized, B: Binding + ?Sized> Drop for Lich<T, B> {
    fn drop(&mut self) {
        self.0.try_sever();
    }
}

impl<B: Binding + ?Sized> Drop for Soul<'_, B> {
    fn drop(&mut self) {
        self.0.sever();
    }
}

fn redeem<'a, T: ?Sized + 'a, B: Binding + ?Sized, const BOUND: bool>(
    lich: Lich<T, B>,
    soul: Soul<'a, B>,
) -> Result<Option<Soul<'a, B>>, Pair<'a, T, B>> {
    if B::are_bound(&lich.0, &soul.0) {
        let mut lich = ManuallyDrop::new(lich);
        unsafe { drop_in_place(&mut lich.0) };
        if BOUND && B::is_life_bound(&soul.0) {
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
