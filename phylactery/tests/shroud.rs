#![cfg(feature = "shroud")]

use core::{
    fmt::{Debug, Display},
    str::FromStr,
};
use phylactery::{shroud, shroud::Shroud};

#[shroud]
pub trait Simple {}

pub fn simple<S: Simple>(simple: &S) {
    <dyn Simple>::shroud(simple);
}

#[shroud(..)]
pub trait Default {}

pub fn default<D: Default + Send + Sync + Unpin>(default: &D) {
    <dyn Default>::shroud(default);
    <dyn Default + Send>::shroud(default);
    <dyn Default + Sync>::shroud(default);
    <dyn Default + Unpin>::shroud(default);
    <dyn Default + Send + Unpin>::shroud(default);
    <dyn Default + Sync + Unpin>::shroud(default);
    <dyn Default + Send + Sync + Unpin>::shroud(default);

    <dyn Default>::shroud(default as &dyn Default);
    <dyn Default + Send>::shroud(default as &(dyn Default + Send));
    <dyn Default + Sync>::shroud(default as &(dyn Default + Sync));
    <dyn Default + Unpin>::shroud(default as &(dyn Default + Unpin));
    <dyn Default + Send + Unpin>::shroud(default as &(dyn Default + Send + Unpin));
    <dyn Default + Sync + Unpin>::shroud(default as &(dyn Default + Sync + Unpin));
    <dyn Default + Send + Sync + Unpin>::shroud(default as &(dyn Default + Send + Sync + Unpin));
}

#[shroud]
#[shroud(Send)]
#[shroud(Sync)]
#[shroud(Send, Sync)]
#[shroud(A = usize, Self)]
pub trait Complex<'a, T: Debug, U: FromStr + 'a, const N: usize>: Simple
where
    for<'b> &'b T: Display,
{
    type A;
}

pub fn complex<
    'a,
    T: Debug,
    U: FromStr + 'a,
    const N: usize,
    C: Complex<'a, T, U, N> + Send + Sync,
>(
    complex: &C,
) where
    for<'b> &'b T: Display,
{
    <dyn Complex<T, U, N, A = C::A>>::shroud(complex);
    <dyn Complex<T, U, N, A = C::A> + Send>::shroud(complex);
    <dyn Complex<T, U, N, A = C::A> + Send + Sync>::shroud(complex);
    <dyn Complex<T, U, N, A = C::A> + Sync>::shroud(complex);
}
