#![cfg(feature = "shroud")]

use core::{
    fmt::{Debug, Display},
    ptr::NonNull,
    str::FromStr,
};
use phylactery::shroud::{Shroud, shroud};

#[shroud]
pub trait Simple {}

pub fn simple<S: Simple>(simple: NonNull<S>) {
    <dyn Simple>::shroud(simple);
}

#[shroud(Send, Sync, Unpin, ..)]
#[shroud(Self, Send, Sync, Unpin, ..)]
pub trait Combine {}

pub fn combine<T: Combine + Send + Sync + Unpin>(combine: NonNull<T>) {
    <dyn Combine>::shroud(combine);
    <dyn Combine + Send>::shroud(combine);
    <dyn Combine + Sync>::shroud(combine);
    <dyn Combine + Unpin>::shroud(combine);
    <dyn Combine + Send + Unpin>::shroud(combine);
    <dyn Combine + Sync + Unpin>::shroud(combine);
    <dyn Combine + Send + Sync + Unpin>::shroud(combine);

    <dyn Combine>::shroud(combine as NonNull<dyn Combine>);
    <dyn Combine + Send>::shroud(combine as NonNull<dyn Combine + Send>);
    <dyn Combine + Sync>::shroud(combine as NonNull<dyn Combine + Sync>);
    <dyn Combine + Unpin>::shroud(combine as NonNull<dyn Combine + Unpin>);
    <dyn Combine + Send + Unpin>::shroud(combine as NonNull<dyn Combine + Send + Unpin>);
    <dyn Combine + Sync + Unpin>::shroud(combine as NonNull<dyn Combine + Sync + Unpin>);
    <dyn Combine + Send + Sync + Unpin>::shroud(
        combine as NonNull<dyn Combine + Send + Sync + Unpin>,
    );
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
    complex: NonNull<C>,
) where
    for<'b> &'b T: Display,
{
    <dyn Complex<T, U, N, A = C::A>>::shroud(complex);
    <dyn Complex<T, U, N, A = C::A> + Send>::shroud(complex);
    <dyn Complex<T, U, N, A = C::A> + Send + Sync>::shroud(complex);
    <dyn Complex<T, U, N, A = C::A> + Sync>::shroud(complex);
}
