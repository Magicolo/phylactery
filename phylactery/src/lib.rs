#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "atomic")]
pub mod atomic;
#[cfg(feature = "cell")]
pub mod cell;
pub mod lich;
pub mod shroud;
pub mod soul;

/// Represents a kind of [`Binding`] for a [`Soul`](soul::Soul) and a
/// [`Lich`](lich::Lich).
///
/// # Safety
/// The implementors must implement a reference counter and the `sever` behavior
/// which ensures that captured lifetimes can not be accessed anymore. A wrong
/// implementation can lead to undefined behavior.
///
/// See [`Cell`](cell::Cell) and [`Atomic`](atomic::Atomic) as implementation
/// examples.
pub unsafe trait Binding {
    const NEW: Self;
    /// Attempts to sever the binding between the [`Soul`](soul::Soul) and all
    /// of its [`Lich`](lich::Lich)es. Called when the [`Soul`](soul::Soul)
    /// is [`sever`](soul::Soul::sever)ed or when it is dropped.
    /// - When `FORCE = true`, the severance **must** have completed when this
    ///   call returns and no bound [`Lich`](lich::Lich) must be accessible.
    /// - When `FORCE = false`, the severance is allowed to fail.
    ///
    /// Returns `true` if the severance was successful.
    fn sever<const FORCE: bool>(&self) -> bool;
    /// Called when the last [`Lich`](lich::Lich) is dropped.
    fn redeem(&self);
    /// Returns the current reference count.
    fn count(&self) -> u32;
    /// Increments the reference count by 1. Called when a [`Lich`](lich::Lich)
    /// is [`bind`](soul::Soul::bind)ed or when it is cloned.
    ///
    /// Returns the old reference count (pre-increment).
    fn increment(&self) -> u32;
    /// Decrements the reference count by 1. Called when a [`Lich`](lich::Lich)
    /// is [`redeem`](soul::Soul::redeem)ed or when it is dropped.
    ///
    /// Returns the old reference count (pre-decrement).
    fn decrement(&self) -> u32;
    /// Returns `true` if the `*const Self` pointer has become invalid. An
    /// implementation that *can* return `true` may leave the reference
    /// counter to a non-zero value and must be handled accordingly.
    fn bail(this: *const Self, drop: bool) -> bool;
}

#[allow(dead_code)]
mod fails {
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
        use phylactery::atomic::Soul;
        use std::thread::spawn;

        let soul = pin!(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send>();
        spawn(move || lich());
    });
}
