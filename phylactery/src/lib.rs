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
    /// Attempts to sever the binding between the [`soul::Soul`] and all of its
    /// [`lich::Lich`]es. Called when the [`soul::Soul`] is
    /// [`soul::Soul::sever`]ed or when it is dropped.
    /// - When `FORCE = true`, the severance **must** have completed when this
    ///   call returns and no bound [`lich::Lich`] must be accessible.
    /// - When `FORCE = false`, the severance is allowed to fail.
    ///
    /// Returns `true` if the severance was successful.
    fn sever<const FORCE: bool>(&self) -> bool;
    /// Called when the last [`lich::Lich`] is dropped.
    fn redeem(&self);
    /// Returns the current reference count.
    fn count(&self) -> u32;
    /// Increments the reference count by 1. Called when a [`lich::Lich`] is
    /// [`soul::Soul::bind`]ed or when it is cloned.
    ///
    /// Returns the old reference count (pre-increment).
    fn increment(&self) -> u32;
    /// Decrements the reference count by 1. Called when a [`lich::Lich`] is
    /// [`soul::Soul::redeem`]ed or when it is dropped.
    ///
    /// Returns the old reference count (pre-decrement).
    fn decrement(&self) -> u32;
}

#[cfg(not(feature = "std"))]
static PANIC: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

fn is_panicking() -> bool {
    #[cfg(feature = "std")]
    return std::thread::panicking();

    #[cfg(not(feature = "std"))]
    return PANIC.load(core::sync::atomic::Ordering::Relaxed);
}

fn panic(value: u32) -> bool {
    #[cfg(feature = "std")]
    if std::thread::panicking() {
        return false;
    }

    #[cfg(not(feature = "std"))]
    if PANIC.swap(true, core::sync::atomic::Ordering::Relaxed) {
        return false;
    }

    if value <= 1 {
        panic!("'{value}' `Lich<T>` has not been redeemed")
    } else {
        panic!("'{value}' `Lich<T>`es have not been redeemed")
    }
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
