#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "cell")]
pub mod cell;
mod lich;
#[cfg(feature = "lock")]
pub mod lock;
pub mod shroud;
mod soul;

/// Represents a kind of [`Binding`] for a [`soul::Soul`] and a [`lich::Lich`].
///
/// # Safety
/// The implementors must implement a reference counter and the `sever` behavior
/// which ensures that captured lifetimes can not be accessed anymore. A wrong
/// implementation can lead to undefined behavior.
///
/// See [`cell::Cell`] and [`lock::Lock`] as implementation examples.
pub unsafe trait Binding {
    const NEW: Self;
    fn sever<const FORCE: bool>(&self) -> bool;
    fn redeem(&self);
    fn count(&self) -> u32;
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
        use core::{cell::RefCell, pin::pin};
        use phylactery::cell::Soul;

        let cell = RefCell::new(String::new());
        let function = move |letter| cell.borrow_mut().push(letter);
        let soul = Soul::new(&function);
        drop(function);
    });

    fail!(can_not_clone_soul, {
        use core::{cell::RefCell, pin::pin};
        use phylactery::cell::Soul;

        let cell = RefCell::new(String::new());
        let soul = Soul::new(move |letter| cell.borrow_mut().push(letter));
        <Soul<_> as Clone>::clone(&soul);
    });

    fail!(can_not_send_cell_to_thread, {
        use core::pin::pin;
        use phylactery::cell::Soul;
        use std::thread::spawn;

        let soul = pin!(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send + Sync>();
        spawn(move || lich());
    });

    fail!(can_not_send_unsync_to_thread, {
        use core::pin::pin;
        use phylactery::lock::Soul;
        use std::thread::spawn;

        let soul = pin!(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send>();
        spawn(move || lich());
    });
}
