#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "cell")]
pub mod cell;
mod lich;
#[cfg(feature = "lock")]
pub mod lock;
pub mod shroud;
mod soul;

pub unsafe trait Bind {
    const NEW: Self;
    fn sever<const FORCE: bool>(&self) -> bool;
    fn redeem(&self);
    fn bindings(&self) -> u32;
    fn increment(&self) -> u32;
    fn decrement(&self) -> u32;
}

#[allow(dead_code)]
mod tests {
    macro_rules! fail {
        ($function: ident, $block: block) => {
            #[doc = concat!("```compile_fail\n", stringify!($block), "\n```")]
            const fn $function() {}
        };
    }

    fail!(can_not_drop_while_soul_lives, {
        use core::cell::RefCell;
        use phylactery::raw::ritual;

        let value = String::new();
        let cell = RefCell::new(value);
        let function = move |letter| cell.borrow_mut().push(letter);
        let (lich, soul) = ritual::<_, dyn Fn(char)>(&function);
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

    fail!(can_not_send_raw_unsync_to_thread, {
        use phylactery::raw::ritual;
        use std::thread::spawn;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn() + Send>(&function);
        spawn(move || lich);
    });

    fail!(can_not_create_default_raw_lich, {
        use phylactery::raw::Lich;
        Lich::<dyn Fn()>::default();
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

    fail!(can_not_create_default_atomic_lich, {
        use phylactery::atomic::Lich;
        Lich::<dyn Fn()>::default();
    });
}
