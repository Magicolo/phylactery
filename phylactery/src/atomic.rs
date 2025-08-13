//! `unsafe`-free, allocation-free, thread-safe, [`Clone`]able, `#[no_std]`-
//! compatible lifetime extension using an [`AtomicU32`] reference counter.
//!
//! This module provides the [`Atomic`] [`Binding`] implementation, which uses
//! an [`AtomicU32`] as a reference counter to track the number of active
//! [`Lich<T>`] clones. It does not require heap allocation, but it does require
//! the user to provide a mutable reference to a `u32` to use as the
//! reference counter.

use crate::{Binding, Sever, TrySever, shroud::Shroud};
use atomic_wait::{wait, wake_one};
use core::{
    borrow::Borrow,
    ptr::{NonNull, addr_eq},
    sync::atomic::{AtomicU32, Ordering},
};

pub struct Atomic;
pub type Soul<'a> = crate::Soul<'a, Atomic>;
pub type Lich<T> = crate::Lich<T, Atomic>;
pub type Pair<'a, T> = crate::Pair<'a, T, Atomic>;
pub struct Data<T: ?Sized>(NonNull<T>, NonNull<AtomicU32>);
pub struct Life<'a>(&'a AtomicU32);

unsafe impl<'a, T: ?Sized + 'a> Send for Data<T> where &'a T: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Data<T> where &'a T: Sync {}

impl<T: ?Sized> TrySever for Data<T> {
    fn try_sever(&mut self) -> Option<bool> {
        None
    }
}

impl<T: ?Sized> Clone for Data<T> {
    fn clone(&self) -> Self {
        unsafe { self.1.as_ref() }.fetch_add(1, Ordering::Relaxed);
        Self(self.0, self.1)
    }
}

impl<T: ?Sized> Drop for Data<T> {
    fn drop(&mut self) {
        let atomic = unsafe { self.1.as_ref() };
        if atomic.fetch_sub(1, Ordering::Release) == 1 {
            wake_one(atomic);
        }
    }
}

impl Sever for Life<'_> {
    fn sever(&mut self) -> bool {
        sever::<true>(self.0).is_some_and(|value| value)
    }
}

impl TrySever for Life<'_> {
    fn try_sever(&mut self) -> Option<bool> {
        sever::<false>(self.0)
    }
}

impl Binding for Atomic {
    type Data<T: ?Sized> = Data<T>;
    type Life<'a> = Life<'a>;

    fn are_bound<T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'_>) -> bool {
        addr_eq(data.1.as_ptr(), life.0)
    }

    fn is_life_bound(life: &Self::Life<'_>) -> bool {
        bound(life.0)
    }

    fn is_data_bound<T: ?Sized>(data: &Self::Data<T>) -> bool {
        bound(unsafe { data.1.as_ref() })
    }
}

impl<T: ?Sized> Borrow<T> for Lich<T> {
    fn borrow(&self) -> &T {
        self.borrow()
    }
}

impl<T: ?Sized> Lich<T> {
    /// This borrow is safe and always succeeds because the [`Soul<'a>`]'s
    /// [`Drop`] implementation will block until all [`Lich<T>`] clones (and
    /// therefore all borrows) are gone.
    #[allow(clippy::should_implement_trait)]
    pub fn borrow(&self) -> &T {
        unsafe { self.0.0.as_ref() }
    }
}

/// Binds the lifetime of `value` to a [`Lich<T>`] and [`Soul<'a>`] pair, using
/// the provided `location` as storage for the reference count.
///
/// The `location` must have a lifetime `'a` that is at least as long as the
/// `value`'s borrow. It will be initialized to `1` and may end with any
/// value.
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized>(
    value: &'a T,
    location: &'a mut u32,
) -> Pair<'a, S> {
    *location = 1;
    // # Safety
    // `location` is trivially valid as an `AtomicU32` and since it is a
    // mutable borrow, it is exclusively owned by this function
    let count = unsafe { core::sync::atomic::AtomicU32::from_ptr(location) };
    let pointer = unsafe { NonNull::new_unchecked(count as *const _ as *mut _) };
    (
        crate::Lich(Data(S::shroud(value), pointer)),
        crate::Soul(Life(count)),
    )
}

/// Safely disposes of a [`Lich<T>`] and [`Soul<'a>`] pair.
///
/// If the provided [`Lich<T>`] and [`Soul<'a>`] are bound together, they are
/// consumed and [`Ok`] is returned with the [`Soul<'a>`] if there are other
/// live [`Lich<T>`] clones. If they are not bound together, [`Err`] is
/// returned with the pair.
///
/// If the [`Lich<T>`] and [`Soul<'a>`] are simply dropped, the [`Soul<'a>`]'s
/// [`Drop`] implementation will block until all [`Lich<T>`] clones are
/// dropped, ensuring safety. While not strictly necessary, using [`redeem`] is
/// good practice for explicit cleanup.
pub fn redeem<'a, T: ?Sized + 'a>(
    lich: Lich<T>,
    soul: Soul<'a>,
) -> Result<Option<Soul<'a>>, Pair<'a, T>> {
    crate::redeem::<_, _, true>(lich, soul)
}

fn sever<const WAIT: bool>(count: &AtomicU32) -> Option<bool> {
    loop {
        match count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
            Ok(0) => break Some(true),
            Ok(u32::MAX) | Err(u32::MAX) => break Some(false),
            Ok(value) | Err(value) if WAIT => wait(count, value),
            Ok(_) | Err(_) => break None,
        }
    }
}

fn bound(count: &AtomicU32) -> bool {
    let count = count.load(Ordering::Acquire);
    count > 0 && count < u32::MAX
}
