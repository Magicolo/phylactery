//! Zero-cost, `unsafe`, allocation-free, thread-safe, `#[no_std]`-compatible
//! lifetime extension.
//!
//! This module provides the [`Raw`] [`Binding`] implementation, which is the
//! most performant but also the most dangerous variant. It offers a zero-cost
//! abstraction, meaning it introduces no heap allocations or reference-counting
//! overhead. The [`Lich<T>`] and [`Soul<P>`] are simple new-type wrappers
//! around raw pointers.

use crate::{pointer::Pointer, shroud::Shroud};
use core::{
    mem::forget,
    ptr::{self, NonNull, read},
};

pub struct Lich<T: ?Sized>(NonNull<T>);
pub struct Soul<P: ?Sized>(P);
type Pair<T, P> = (Lich<T>, Soul<P>);

unsafe impl<T: ?Sized> Send for Lich<T> where for<'a> &'a T: Send {}
unsafe impl<T: ?Sized> Sync for Lich<T> where for<'a> &'a T: Sync {}

impl<T: ?Sized> Lich<T> {
    /// # Safety
    ///
    /// The caller must ensure that the corresponding [`Soul<P>`] is still
    /// alive and in scope. Dropping the [`Soul<P>`] while this borrow is
    /// active will invalidate the pointer, leading to a `use-after-free`
    /// vulnerability.
    ///
    /// The [`Raw`] variant offers no runtime checks to prevent this. It is the
    /// caller's responsibility to uphold this safety contract.
    pub unsafe fn borrow(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        sever_panic();
    }
}

impl<P: ?Sized> Drop for Soul<P> {
    fn drop(&mut self) {
        sever_panic();
    }
}

/// Binds the lifetime of `value` to a [`Lich<T>`] and [`Soul<P>`] pair.
///
/// The returned [`Lich<T>`] and [`Soul<P>`] will both **[`panic!`] on
/// drop** and **must** be sent to [`redeem`] to be disposed.
pub fn ritual<P: Pointer, S: Shroud<P::Target> + ?Sized>(pointer: P) -> Pair<S, P> {
    (Lich(S::shroud(pointer.pointer())), Soul(pointer))
}

/// Safely disposes of a [`Lich<T>`] and [`Soul<P>`] pair.
///
/// This function is **required** for this variant. It safely disposes of the
/// pair, preventing their [`Drop`] implementations from [`panic!`]ing.
///
/// If the provided [`Lich<T>`] and [`Soul<P>`] are bound together, they are
/// consumed and [`Ok`] is returned with the original pointer. If they are not
/// bound together, [`Err`] is returned with the pair.
pub fn redeem<T: ?Sized, P: Pointer>(lich: Lich<T>, soul: Soul<P>) -> Result<P, Pair<T, P>> {
    if ptr::addr_eq(lich.0.as_ptr(), soul.0.pointer()) {
        let pointer = unsafe { read(&soul.0) };
        forget(soul);
        forget(lich);
        Ok(pointer)
    } else {
        Err((lich, soul))
    }
}

fn sever_panic() -> bool {
    #[cfg(feature = "std")]
    if std::thread::panicking() {
        return false;
    }

    #[cfg(not(feature = "std"))]
    {
        use core::sync::atomic::{AtomicBool, Ordering};

        static PANIC: AtomicBool = AtomicBool::new(false);
        if PANIC.swap(true, Ordering::Relaxed) {
            return false;
        }
    }

    panic!("this `Lich<T>` must be redeemed")
}
