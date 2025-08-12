//! Lifetime extension utilities.
//!
//! The [`Shroud<T>`] trait and the associated [`crate::shroud!`] macro are the
//! core of this library's lifetime extension mechanism. They provide a way to
//! erase the lifetime of a reference by converting it into a raw pointer, which
//! can then be safely managed by the [`crate::Lich<T, B>`] and
//! [`crate::Soul<'a, B>`] pairs.
//!
//! # Usage
//!
//! The [`crate::shroud!`] macro is used to implement the [`Shroud<T>`] trait
//! for a target trait object.
//!
//! ```
//! use phylactery::shroud;
//!
//! pub trait Trait {
//!     fn do_something(&self);
//! }
//!
//! // This implements `shroud::Shroud<dyn Trait>` for `dyn Trait`.
//! shroud!(Trait);
//! ```
//!
//! ```
//! use phylactery::shroud;
//!
//! pub trait OtherTrait {
//!     fn do_something_else(&self);
//! }
//!
//! // This also implements it for `Send` and `Sync` variants.
//! // e.g. `shroud::Shroud<dyn OtherTrait + Send>` for `dyn OtherTrait + Send`
//! shroud!(OtherTrait +);
//! ```
use core::ptr::NonNull;

/// A trait for erasing the lifetime of a reference.
///
/// This trait provides the `unsafe` underpinning for the entire library. It
/// allows converting a reference `&T` into a `'static` raw pointer
/// `NonNull<Self>`, effectively "shrouding" its original lifetime. The lifetime
/// is then tracked separately by a [`Soul<'a, B>`].
///
/// This trait is not intended to be implemented manually. Instead, the
/// [`crate::shroud!`] macro should be used, which will correctly implement it
/// for a trait and its variations (e.g., `+ Send`, `+ Sync`).
pub trait Shroud<T: ?Sized> {
    /// Erases the lifetime of the given reference.
    ///
    /// This is safe to call, but using the returned pointer is `unsafe` as its
    /// lifetime is not tracked by the type system. The [`crate::Lich<T, B>`]
    /// and [`crate::Soul<'a, B>`] mechanism in this library provides a safe way
    /// to manage this.
    fn shroud(from: &T) -> NonNull<Self>;
}

/// A macro to implement the [`Shroud<T>`] trait for a given trait object.
///
/// This is the recommended way to implement the [`Shroud<T>`] trait. It handles
/// the implementation for the base trait, as well as for common variations
/// with marker traits like `Send` and `Sync`.
///
/// # Usage
///
/// ```
/// # use phylactery::shroud;
/// pub trait Trait {}
///
/// // Implement `Shroud` for `dyn Trait`.
/// shroud!(Trait);
/// ```
///
/// It is also possible to specify marker traits:
///
/// ```
/// # use phylactery::shroud;
/// # pub trait Trait {}
/// // Implement `Shroud` for `dyn Trait + Send + Sync`.
/// shroud!(Trait + Send + Sync);
/// ```
///
/// The `+` syntax is a convenient shorthand to implement for all common
/// combinations of `Send`, `Sync` and `Unpin`.
///
/// ```
/// # use phylactery::shroud;
/// # pub trait OtherTrait {}
/// // Implements `Shroud` for `dyn OtherTrait`, `dyn OtherTrait + Send`,
/// // `dyn OtherTrait + Sync`, `dyn OtherTrait + Send + Sync`, etc.
/// shroud!(OtherTrait +);
/// ```
#[macro_export]
macro_rules! shroud {
    ($type: ident) => {
        shroud!(@TRAIT { type: $type, generics: (), traits: () });
    };
    ($type: ident +) => {
        shroud!(@TRAIT { type: $type, generics: (), traits: ((Send), (Sync), (Unpin), (Send, Sync), (Send, Unpin), (Sync, Unpin), (Send, Sync, Unpin)) });
    };
    ($type: ident $(+ $trait: ident)+) => {
        shroud!(@TRAIT { type: $type, generics: (), traits: (($($trait),*)) });
    };
    ($type: ident<$($generic: ident),* $(,)?>) => {
        shroud!(@TRAIT { type: $type, generics: ($($generic),*), traits: () });
    };
    ($type: ident<$($generic: ident),* $(,)?> +) => {
        shroud!(@TRAIT { type: $type, generics: ($($generic),*), traits: ((Send), (Sync), (Unpin), (Send, Sync), (Send, Unpin), (Sync, Unpin), (Send, Sync, Unpin)) });
    };
    ($type: ident<$($generic: ident),* $(,)?> $(+ $trait: ident)+) => {
        shroud!(@TRAIT { type: $type, generics: ($($generic),*), traits: (($($trait),*)) });
    };
    (@TRAIT { type: $type: ident, generics: $generics: tt, traits: () $(,)? }) => {
        shroud!(@IMPLEMENT { type: $type, generics: $generics, traits: () });
    };
    (@TRAIT { type: $type: ident, generics: $generics: tt, traits: ($trait: tt $(, $traits: tt)*) $(,)? }) => {
        shroud!(@TRAIT { type: $type, generics: $generics, traits: ($($traits),*) });
        shroud!(@IMPLEMENT { type: $type, generics: $generics, traits: $trait });
    };
    (@IMPLEMENT { type: $type: ident, generics: ($($generic: ident),*), traits: ($($trait: path),*) $(,)? }) => {
        impl<$($generic,)*> $crate::shroud::Shroud<dyn $type<$($generic),*> $(+ $trait)*> for dyn $type<$($generic),*> $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: &(dyn $type<$($generic),*> $(+ $trait)*)) -> ::core::ptr::NonNull<Self> {
                // # Safety
                // The pointer is trivially non-null as per rust's reference guarantees.
                unsafe { ::core::ptr::NonNull::new_unchecked(from as *const _ as *const Self as *mut _) }
            }
        }

        impl<$($generic,)* TConcrete: $type<$($generic),*> $(+ $trait)*> $crate::shroud::Shroud<TConcrete> for dyn $type<$($generic),*> $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: &TConcrete) -> ::core::ptr::NonNull<Self> {
                unsafe { ::core::ptr::NonNull::new_unchecked(from as *const TConcrete as *const Self as *mut _) }
            }
        }
    };
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
        impl<$($parameter,)* $return> $crate::shroud::Shroud<dyn $function($($parameter),*) -> $return $(+ $trait)*> for dyn $function($($parameter),*) -> $return $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: &(dyn $function($($parameter),*) -> $return $(+ $trait)*)) -> ::core::ptr::NonNull<Self> {
                unsafe { ::core::ptr::NonNull::new_unchecked(from as *const _ as *const Self as *mut _) }
            }
        }

        impl<$($parameter,)* $return, TConcrete: $function($($parameter),*) -> $return $(+ $trait)*> $crate::shroud::Shroud<TConcrete> for dyn $function($($parameter),*) -> $return $(+ $trait)* {
            #[inline(always)]
            fn shroud(from: &TConcrete) -> ::core::ptr::NonNull<Self> {
                unsafe { ::core::ptr::NonNull::new_unchecked(from as *const _ as *const Self as *mut _) }
            }
        }
    };
}

shroud_fn!(Fn(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
