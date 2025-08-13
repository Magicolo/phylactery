//! Zero-cost, `unsafe`, allocation-free, thread-safe,`#[no_std]`-compatible
//! lifetime extension.
//!
//! This module provides the [`Raw`] [`Binding`] implementation, which is the
//! most performant but also the most dangerous variant. It offers a zero-cost
//! abstraction, meaning it introduces no heap allocations or reference counting
//! overhead. The [`Lich<T>`] and [`Soul<'a>`] are simple new type wrappers
//! around raw pointers.

use crate::{Binding, Sever, TrySever, shroud::Shroud};
use core::{
    marker::PhantomData,
    ptr::{self, NonNull},
};

pub struct Raw;
pub type Soul<'a> = crate::Soul<'a, Raw>;
pub type Lich<T> = crate::Lich<T, Raw>;
pub type Pair<'a, T> = crate::Pair<'a, T, Raw>;
pub struct Data<T: ?Sized>(NonNull<T>);
pub struct Life<'a>(NonNull<()>, PhantomData<&'a ()>);

unsafe impl<'a, T: ?Sized + 'a> Send for Data<T> where &'a T: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Data<T> where &'a T: Sync {}

impl<T: ?Sized> TrySever for Data<T> {
    fn try_sever(&mut self) -> Option<bool> {
        Some(sever_panic())
    }
}

impl Sever for Life<'_> {
    fn sever(&mut self) -> bool {
        sever_panic()
    }
}

impl Binding for Raw {
    type Data<T: ?Sized> = Data<T>;
    type Life<'a> = Life<'a>;

    /// This function can return false positives if the same `&'a T` is bound
    /// twice and the `Self::Data<T>` of the first binding is checked against
    /// the `Self::Life<'a>` of the second.
    fn are_bound<'a, T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'a>) -> bool {
        ptr::addr_eq(data.0.as_ptr(), life.0.as_ptr())
    }

    /// `Self::Life<'a>` is always bounded until redeemed.
    fn is_life_bound(_: &Self::Life<'_>) -> bool {
        true
    }

    /// `Self::Data<T>` is always bounded until redeemed.
    fn is_data_bound<T: ?Sized>(_: &Self::Data<T>) -> bool {
        true
    }
}

impl<T: ?Sized> Lich<T> {
    /// # Safety
    ///
    /// The caller must ensure that the corresponding [`Soul<'a>`] is still
    /// alive and in scope. Dropping the [`Soul<'a>`] while this borrow is
    /// active will invalidate the pointer, leading to a **use-after-free**
    /// vulnerability.
    ///
    /// The [`Raw`] variant offers no runtime checks to prevent this. It is the
    /// caller's responsibility to uphold this safety contract.
    pub unsafe fn borrow(&self) -> &T {
        unsafe { self.0.0.as_ref() }
    }
}

/// Binds the lifetime of `value` to a [`Lich<T>`] and [`Soul<'a>`] pair.
///
/// The returned [`Lich<T>`] and [`Soul<'a>`] will both **[`panic`] on drop**
/// and **must** be sent to [`redeem`] to be disposed.
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> Pair<'a, S> {
    let pointer = S::shroud(value);
    (
        crate::Lich(Data(pointer)),
        crate::Soul(Life(pointer.cast(), PhantomData)),
    )
}

/// Safely disposes of a [`Lich<T>`] and [`Soul<'a>`] pair.
///
/// This function is **required** for the [`Raw`] variant. It safely disposes of
/// the pair, preventing their [`Drop`] implementations from [`panic`]king.
///
/// If the provided [`Lich<T>`] and [`Soul<'a>`] are bound together, they are
/// consumed and [`Ok`] is returned. If they are not bound together, [`Err`] is
/// returned with the pair.
pub fn redeem<'a, T: ?Sized + 'a>(lich: Lich<T>, soul: Soul<'a>) -> Result<(), Pair<'a, T>> {
    crate::redeem::<_, _, false>(lich, soul).map(|_| {})
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

    panic!("this `Lich<T, Raw>` must be redeemed")
}
