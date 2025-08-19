//! The [`Shroud<T>`] trait and the associated [`shroud`] macro are the core
//! of this library's lifetime extension mechanism. They provide a way to erase
//! the lifetime of a reference by converting it into a raw pointer or a dynamic
//! trait, which can then be safely managed by a [`crate::Lich<T, B>`] and
//! [`crate::Soul<'a, B>`] pair.

use core::ptr::NonNull;
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
/// use phylactery::shroud::shroud;
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

/// A trait for erasing the lifetime of a reference and converting it to a
/// dynamic trait pointer.
///
/// Note that it is already implemented for `Fn(T0, .., T7) -> T` and its
/// combinations with [`Send`], [`Sync`] and [`Unpin`].
///
/// See the [`shroud`] macro for convenient implementation.
pub unsafe trait Shroud<T: ?Sized> {
    fn shroud(from: *const T) -> NonNull<Self>;
}

macro_rules! shroud_fn {
    ($function: ident($(,)?) -> $return: ident) => {
        shroud_fn!(@TRAIT {
            function: $function,
            parameters: (),
            return: $return,
            traits: ((Send), (Sync), (Unpin), (Send, Sync), (Send, Unpin), (Sync, Unpin), (Send, Sync, Unpin)),
        });
    };
    ($function: ident($parameter: ident $(, $parameters: ident)* $(,)?) -> $return: ident) => {
        shroud_fn!($function($($parameters),*) -> $return);
        shroud_fn!(@TRAIT {
            function: $function,
            parameters: ($parameter $(, $parameters)*),
            return: $return,
            traits: ((Send), (Sync), (Unpin), (Send, Sync), (Send, Unpin), (Sync, Unpin), (Send, Sync, Unpin)),
        });
    };
    (@TRAIT { function: $function: ident, parameters: $parameters: tt, return: $return: ident, traits: () $(,)? }) => {
        shroud_fn!(@IMPLEMENT { function: $function, parameters: $parameters, return: $return, traits: () });
    };
    (@TRAIT { function: $function: ident, parameters: $parameters: tt, return: $return: ident, traits: ($trait: tt $(, $traits: tt)*) $(,)? }) => {
        shroud_fn!(@TRAIT { function: $function, parameters: $parameters, return: $return, traits: ($($traits),*) });
        shroud_fn!(@IMPLEMENT { function: $function, parameters: $parameters, return: $return, traits: $trait });
    };
    (@IMPLEMENT { function: $function: ident, parameters: ($($parameter: ident),*), return: $return: ident, traits: ($($trait: path),*) $(,)? }) => {
        unsafe impl<$($parameter,)* $return> $crate::shroud::Shroud<dyn $function($($parameter),*) -> $return $(+ $trait)*> for dyn $function($($parameter),*) -> $return $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: *const (dyn $function($($parameter),*) -> $return $(+ $trait)*)) -> ::core::ptr::NonNull<Self> {
                ::core::ptr::NonNull::new(from as *const _ as *const Self as *mut _).expect("null pointer")
            }
        }

        unsafe impl<$($parameter,)* $return, TConcrete: $function($($parameter),*) -> $return $(+ $trait)*> $crate::shroud::Shroud<TConcrete> for dyn $function($($parameter),*) -> $return $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: *const TConcrete) -> ::core::ptr::NonNull<Self> {
                ::core::ptr::NonNull::new(from as *const _ as *const Self as *mut _).expect("null pointer")
            }
        }
    };
}

shroud_fn!(Fn(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
