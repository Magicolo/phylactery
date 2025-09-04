#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

mod lich;
mod shroud;
mod soul;

pub use lich::Lich;
/// A convenience macro to implement the [`Shroud<T>`] trait for a given trait.
/// The macro is applied to a trait directly because it will derive blanket
/// implementations of [`Shroud<T>`] for all `T: Trait`. It can also handle
/// implementing [`Shroud<T>`] for all combinations of marker traits [`Send`],
/// [`Sync`], and [`Unpin`] (e.g., `dyn Trait + Send`, `dyn Trait + Sync`, `dyn
/// Trait + Send + Unpin`, etc.).
///
/// # Usage
///
/// ```
/// use core::{
///     fmt::{Debug, Display},
///     str::FromStr,
/// };
/// use phylactery::shroud;
///
/// // Generates `impl<T: Simple> Shroud<T> for dyn Simple`.
/// #[shroud]
/// pub trait Simple {}
///
/// // The `..` will generate implementations for all combinations of the specified
/// // traits. In this case `dyn Combine`, `dyn Combine + Send`, `dyn Combine + Sync + Unpin`,
/// // `dyn Combine + Send + Sync + Unpin`, etc. Be wary that the number of
/// // implementations can be very large if used with many traits.
/// #[shroud(Send, Sync, Unpin, ..)]
/// pub trait Combine {}
///
/// // Instead of `..`, the combinations can be specified manually by adding
/// // multiple `#[shroud]`.
/// #[shroud]
/// #[shroud(Send)]
/// #[shroud(Sync)]
/// #[shroud(Send, Sync)]
/// // `Self` means that the implementation will be for `Shroud<dyn Trait> for
/// // dyn Trait` rather than a blanket implementation. In that case, associated
/// // types must be specified explicitly (here with `A = usize`).
/// #[shroud(Self, A = usize)]
/// pub trait Complex<'a, T: Debug, U: FromStr + 'a, const N: usize>: Simple
/// where
///     for<'b> &'b T: Display,
/// {
///     type A;
/// }
/// ```
#[cfg(feature = "shroud")]
pub use phylactery_macro::shroud;
pub use shroud::Shroud;
pub use soul::Soul;

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
        use phylactery::Soul;

        let cell = RefCell::new(String::new());
        let function = move |letter| cell.borrow_mut().push(letter);
        let soul = Soul::new(&function);
        drop(function);
    });

    fail!(can_not_clone_soul, {
        use core::{cell::RefCell, pin::pin};
        use phylactery::Soul;

        let cell = RefCell::new(String::new());
        let soul = Soul::new(move |letter| cell.borrow_mut().push(letter));
        <Soul<_> as Clone>::clone(&soul);
    });

    fail!(can_not_send_unsync_to_thread, {
        use core::pin::pin;
        use phylactery::Soul;
        use std::thread::spawn;

        let soul = pin!(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send>();
        spawn(move || lich());
    });
}
