use core::ptr::NonNull;

/// A trait for erasing the lifetime of a reference and converting it to a
/// dynamic trait pointer.
///
/// Note that it is already implemented for `Fn(T0, .., T7) -> T` and its
/// combinations with [`Send`], [`Sync`] and [`Unpin`].
///
/// See the [`shroud`](crate::shroud) macro for convenient implementation.
pub trait Shroud<T: ?Sized> {
    fn shroud(from: NonNull<T>) -> NonNull<Self>;
}

#[cfg(feature = "shroud")]
mod implement {
    macro_rules! shroud_ty {
        (use: $use: path, trait: $trait: ident, generics: ($($generic: ident),*), bounds: ($($name: ident : $bound: path),*), associates: ($($associate: ident),*), dynamic: $dynamic: expr) => {
            shroud_ty! { use: $use, trait: $trait, generics: ($($generic),*), bounds: ($($name: $bound),*), associates: ($($associate),*), dynamic: $dynamic, traits: () }
            shroud_ty! { use: $use, trait: $trait, generics: ($($generic),*), bounds: ($($name: $bound),*), associates: ($($associate),*), dynamic: $dynamic, traits: (Send) }
            shroud_ty! { use: $use, trait: $trait, generics: ($($generic),*), bounds: ($($name: $bound),*), associates: ($($associate),*), dynamic: $dynamic, traits: (Sync) }
            shroud_ty! { use: $use, trait: $trait, generics: ($($generic),*), bounds: ($($name: $bound),*), associates: ($($associate),*), dynamic: $dynamic, traits: (Unpin) }
            shroud_ty! { use: $use, trait: $trait, generics: ($($generic),*), bounds: ($($name: $bound),*), associates: ($($associate),*), dynamic: $dynamic, traits: (Send, Sync) }
            shroud_ty! { use: $use, trait: $trait, generics: ($($generic),*), bounds: ($($name: $bound),*), associates: ($($associate),*), dynamic: $dynamic, traits: (Send, Unpin) }
            shroud_ty! { use: $use, trait: $trait, generics: ($($generic),*), bounds: ($($name: $bound),*), associates: ($($associate),*), dynamic: $dynamic, traits: (Sync, Unpin) }
            shroud_ty! { use: $use, trait: $trait, generics: ($($generic),*), bounds: ($($name: $bound),*), associates: ($($associate),*), dynamic: $dynamic, traits: (Send, Sync, Unpin) }
        };
        (use: $use: path, trait: $trait: ident, generics: ($($generic: ident),*), bounds: ($($name: ident : $bound: path),*), associates: (), dynamic: true, traits: ($($traits: path),*) ) => {
            shroud_ty! { use: $use, trait: $trait, generics: ($($generic),*), bounds: ($($name: $bound),*), associates: (), dynamic: false, traits: ($($traits),*) }
            const _: () = {
                use $use;

                #[automatically_derived]
                #[allow(unused_parens)]
                impl<$($generic: ?Sized,)* $($name,)*> $crate::shroud::Shroud<dyn $trait<$($generic,)* $($name,)*> $(+ $traits)*> for dyn $trait<$($generic,)* $($name,)*> $(+ $traits)* where $($name: $bound,)* {
                    #[inline(always)]
                    fn shroud(from: ::core::ptr::NonNull<dyn $trait<$($generic,)* $($name,)*> $(+ $traits)*>) -> ::core::ptr::NonNull<Self> {
                        unsafe {
                            ::core::ptr::NonNull::new_unchecked(::core::mem::transmute::<
                                *mut (dyn $trait<$($generic,)* $($name,)*> $(+ $traits)*),
                                *mut Self
                            >(from.as_ptr() as _))
                        }
                    }
                }
            };
        };
        (use: $use: path, trait: $trait: ident, generics: ($($generic: ident),*), bounds: ($($name: ident : $bound: path),*), associates: ($($associate: ident),*), dynamic: $dynamic: expr, traits: ($($traits: path),*) ) => {
            const _: () = {
                use $use;

                #[automatically_derived]
                #[allow(drop_bounds, dyn_drop, unused_parens)]
                impl<$($generic: ?Sized,)* $($name,)* TConcrete: $trait<$($generic,)* $($name,)*> $(+ $traits)*> $crate::shroud::Shroud<TConcrete> for dyn $trait<$($generic,)* $($name,)* $($associate = TConcrete::$associate,)*> $(+ $traits)* where $($name: $bound,)* {
                    #[inline(always)]
                    fn shroud(from: ::core::ptr::NonNull<TConcrete>) -> ::core::ptr::NonNull<Self> {
                        unsafe {
                            ::core::ptr::NonNull::new_unchecked(::core::mem::transmute::<
                                *mut (dyn $trait<$($generic,)* $($name,)* $($associate = TConcrete::$associate,)*> $(+ $traits)*),
                                *mut Self
                            >(from.as_ptr() as _))
                        }
                    }
                }
            };
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
            #[automatically_derived]
            #[allow(unused_parens)]
            impl<$($parameter,)* $return> $crate::shroud::Shroud<dyn $function($($parameter),*) -> $return $(+ $trait)*> for dyn $function($($parameter),*) -> $return $(+ $trait)* {
                #[inline(always)]
                fn shroud(from: ::core::ptr::NonNull<dyn $function($($parameter),*) -> $return $(+ $trait)*>) -> ::core::ptr::NonNull<Self> {
                    unsafe {
                        ::core::ptr::NonNull::new_unchecked(::core::mem::transmute::<
                            *mut (dyn $function($($parameter),*) -> $return $(+ $trait)*),
                            *mut Self
                        >(from.as_ptr() as _))
                    }
                }
            }

            #[automatically_derived]
            #[allow(unused_parens)]
            impl<$($parameter,)* $return, TConcrete: $function($($parameter),*) -> $return $(+ $trait)*> $crate::shroud::Shroud<TConcrete> for dyn $function($($parameter),*) -> $return $(+ $trait)* {
                #[inline(always)]
                fn shroud(from: ::core::ptr::NonNull<TConcrete>) -> ::core::ptr::NonNull<Self> {
                    unsafe {
                        ::core::ptr::NonNull::new_unchecked(::core::mem::transmute::<
                            *mut (dyn $function($($parameter),*) -> $return $(+ $trait)*),
                            *mut Self
                        >(from.as_ptr() as _))
                    }
                }
            }
        };
    }

    shroud_ty! { use: ::core::any::Any, trait: Any, generics: (), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::borrow::Borrow, trait: Borrow, generics: (T), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::borrow::BorrowMut, trait: BorrowMut, generics: (T), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::cmp::PartialEq, trait: PartialEq, generics: (T), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::cmp::PartialOrd, trait: PartialOrd, generics: (T), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::convert::AsMut, trait: AsMut, generics: (T), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::convert::AsRef, trait: AsRef, generics: (T), bounds: (), associates: (), dynamic: true }
    #[rustversion::since(1.81.0)]
    shroud_ty! { use: ::core::error::Error, trait: Error, generics: (), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::future::Future, trait: Future, generics: (), bounds: (), associates: (Output), dynamic: true }
    shroud_ty! { use: ::core::hash::BuildHasher, trait: BuildHasher, generics: (), bounds: (), associates: (Hasher), dynamic: true }
    shroud_ty! { use: ::core::hash::Hasher, trait: Hasher, generics: (), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::iter::DoubleEndedIterator, trait: DoubleEndedIterator, generics: (), bounds: (), associates: (Item), dynamic: true }
    shroud_ty! { use: ::core::iter::ExactSizeIterator, trait: ExactSizeIterator, generics: (), bounds: (), associates: (Item), dynamic: true }
    shroud_ty! { use: ::core::iter::FusedIterator, trait: FusedIterator, generics: (), bounds: (), associates: (Item), dynamic: true }
    shroud_ty! { use: ::core::iter::Iterator, trait: Iterator, generics: (), bounds: (), associates: (Item), dynamic: true }
    shroud_ty! { use: ::core::marker::Send, trait: Send, generics: (), bounds: (), associates: (), dynamic: true, traits: () }
    shroud_ty! { use: ::core::marker::Sync, trait: Sync, generics: (), bounds: (), associates: (), dynamic: true, traits: () }
    shroud_ty! { use: ::core::marker::Unpin, trait: Unpin, generics: (), bounds: (), associates: (), dynamic: true, traits: () }
    shroud_ty! { use: ::core::panic::RefUnwindSafe, trait: RefUnwindSafe, generics: (), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::panic::UnwindSafe, trait: UnwindSafe, generics: (), bounds: (), associates: (), dynamic: true }
    shroud_ty! { use: ::core::slice::SliceIndex, trait: SliceIndex, generics: (T), bounds: (), associates: (Output), dynamic: true }

    const _: () = {
        shroud_ty! { use: ::core::fmt::Binary, trait: Binary, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::fmt::Debug, trait: Debug, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::fmt::Display, trait: Display, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::fmt::LowerExp, trait: LowerExp, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::fmt::LowerHex, trait: LowerHex, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::fmt::Octal, trait: Octal, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::fmt::Pointer, trait: Pointer, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::fmt::UpperExp, trait: UpperExp, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::fmt::UpperHex, trait: UpperHex, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::fmt::Write, trait: Write, generics: (), bounds: (), associates: (), dynamic: true }
    };

    const _: () = {
        shroud_ty! { use: ::core::ops::AddAssign, trait: AddAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::BitAndAssign, trait: BitAndAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::BitOrAssign, trait: BitOrAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::BitXorAssign, trait: BitXorAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::Deref, trait: Deref, generics: (), bounds: (), associates: (Target), dynamic: true }
        shroud_ty! { use: ::core::ops::DerefMut, trait: DerefMut, generics: (), bounds: (), associates: (Target), dynamic: true }
        shroud_ty! { use: ::core::ops::DivAssign, trait: DivAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::Drop, trait: Drop, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::Index, trait: Index, generics: (T), bounds: (), associates: (Output), dynamic: true }
        shroud_ty! { use: ::core::ops::IndexMut, trait: IndexMut, generics: (T), bounds: (), associates: (Output), dynamic: true }
        shroud_ty! { use: ::core::ops::MulAssign, trait: MulAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::RemAssign, trait: RemAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::ShlAssign, trait: ShlAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::ShrAssign, trait: ShrAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
        shroud_ty! { use: ::core::ops::SubAssign, trait: SubAssign, generics: (), bounds: (T: Sized), associates: (), dynamic: true }
    };

    #[cfg(feature = "std")]
    #[rustversion::before(1.81.0)]
    shroud_ty! { use: ::std::error::Error, trait: Error, generics: (), bounds: (), associates: (), dynamic: true }

    #[cfg(feature = "std")]
    const _: () = {
        shroud_ty! { use: ::std::io::BufRead, trait: BufRead, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::std::io::IsTerminal, trait: IsTerminal, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::std::io::Read, trait: Read, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::std::io::Seek, trait: Seek, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::std::io::Write, trait: Write, generics: (), bounds: (), associates: (), dynamic: true }
        shroud_ty! { use: ::std::string::ToString, trait: ToString, generics: (), bounds: (), associates: (), dynamic: true }
    };

    shroud_fn!(Fn(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
    shroud_fn!(FnMut(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
    shroud_fn!(FnOnce(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
}
