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

#[shroud(Send, Sync, Unpin, ..)]
#[shroud(Self, Send, Sync, Unpin, ..)]
pub trait Combine {}

pub fn default<D: Combine + Send + Sync + Unpin>(default: &D) {
    <dyn Combine>::shroud(default);
    <dyn Combine + Send>::shroud(default);
    <dyn Combine + Sync>::shroud(default);
    <dyn Combine + Unpin>::shroud(default);
    <dyn Combine + Send + Unpin>::shroud(default);
    <dyn Combine + Sync + Unpin>::shroud(default);
    <dyn Combine + Send + Sync + Unpin>::shroud(default);

    <dyn Combine>::shroud(default as &dyn Combine);
    <dyn Combine + Send>::shroud(default as &(dyn Combine + Send));
    <dyn Combine + Sync>::shroud(default as &(dyn Combine + Sync));
    <dyn Combine + Unpin>::shroud(default as &(dyn Combine + Unpin));
    <dyn Combine + Send + Unpin>::shroud(default as &(dyn Combine + Send + Unpin));
    <dyn Combine + Sync + Unpin>::shroud(default as &(dyn Combine + Sync + Unpin));
    <dyn Combine + Send + Sync + Unpin>::shroud(default as &(dyn Combine + Send + Sync + Unpin));
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
