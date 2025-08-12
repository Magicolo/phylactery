//! The [`Shroud<T>`] trait and the associated [`crate::shroud!`] macro are the
//! core of this library's lifetime extension mechanism. They provide a way to
//! erase the lifetime of a reference by converting it into a raw pointer or a
//! dynamic trait, which can then be safely managed by a [`crate::Lich<T, B>`]
//! and [`crate::Soul<'a, B>`] pair.

use core::ptr::NonNull;

/// A trait for erasing the lifetime of a reference and converting it to a
/// dynamic trait pointer.
///
/// Note that it is already implemented for `Fn(T0, .., T7) -> T` and its
/// combinations with [`Send`], [`Sync`] and [`Unpin`].
///
/// See the [`shroud`] macro for convenient implementation.
pub trait Shroud<T: ?Sized> {
    fn shroud(from: &T) -> NonNull<Self>;
}

/// A convenience macro to implement the [`Shroud<T>`] trait for a given trait.
/// It can also handle implementing [`Shroud<T>`] for all combinations of
/// [`Send`], [`Sync`] and [`Unpin`] (ex: `dyn Trait + Send `, `dyn Trait +
/// Sync`, `dyn Trait + Send + Sync`, etc.).
///
///
/// # Usage
///
/// The [`crate::shroud!`] macro is used to implement the [`Shroud<T>`] trait
/// for a target trait object.
///
/// ```
/// use phylactery::shroud;
///
/// pub trait Trait1 { }
/// pub trait Trait2 { }
///
/// // This implements `shroud::Shroud<T: Trait1> for dyn Trait1`.
/// shroud!(Trait1);
/// // This implements `shroud::Shroud<T: Trait2> for dyn Trait2` with all combinations of `Send`, `Sync` and `Unpin`.
/// shroud!(Trait2+);
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
