//! `unsafe`-free, `#[no_std]`-compatible lifetime extension using atomics.
//!
//! This module provides the `atomic` binding, which uses an
//! [`AtomicU32`] as a reference counter to track
//! the number of active [`Lich<T, Atomic>`] clones. It does not require heap
//! allocation, but it does require the user to provide a mutable reference to a
//! `u32` to store the counter.
//!
//! # Trade-offs
//!
//! - **Pros:**
//!   - `unsafe`-free public API.
//!   - `#[no_std]` compatible (with the `atomic-wait` feature).
//!   - [`Lich<T, Atomic>`] can be cloned.
//!   - Can be sent to other threads.
//! - **Cons:**
//!   - Requires the user to provide an `&'a mut u32` for storage.
//!   - If the [`Soul<'a, Atomic>`] is dropped while [`Lich<T, Atomic>`] clones
//!     still exist, the [`Soul<'a, Atomic>`]'s drop implementation will block
//!     until all [`Lich<T, Atomic>`] clones are dropped, which can lead to
//!     deadlocks.
//!
//! # Usage
//!
//! ```
//! use phylactery::{shroud, atomic::{ritual, redeem}};
//!
//! pub trait Trait: Send + Sync {
//!     fn do_it(&self);
//! }
//! shroud!(Trait +);
//!
//! struct Foo(i32);
//! impl Trait for Foo {
//!     fn do_it(&self) {
//!         println!("Value is: {}", self.0);
//!     }
//! }
//!
//! let foo = Foo(42);
//!
//! // A counter is required for the atomic variant.
//! let mut count = 0;
//! let (lich, soul) = ritual::<_, dyn Trait>(&foo, &mut count);
//!
//! let lich_clone = lich.clone();
//! std::thread::spawn(move || {
//!     let f = lich_clone.borrow();
//!     f.do_it();
//! }).join().unwrap();
//!
//! let f = lich.borrow();
//! f.do_it();
//!
//! // It's good practice to redeem the pair, though not strictly required
//! // unless you need to handle the Soul explicitly.
//! redeem(lich, soul).ok().unwrap();
//! ```
use crate::{shroud::Shroud, Binding, Sever, TrySever};
use atomic_wait::{wait, wake_one};
use core::{
    borrow::Borrow,
    ptr::{addr_eq, NonNull},
    sync::atomic::{AtomicU32, Ordering},
};

/// The `atomic` `Binding` variant.
///
/// See the [module-level documentation](self) for more details.
pub struct Atomic;

/// A [`Soul<'a, B>`](crate::Soul) bound to the `atomic` variant.
pub type Soul<'a> = crate::Soul<'a, Atomic>;
/// A [`Lich<T, B>`](crate::Lich) bound to the `atomic` variant.
pub type Lich<T> = crate::Lich<T, Atomic>;
/// A [`Pair<'a, T, B>`](crate::Pair) bound to the `atomic` variant.
pub type Pair<'a, T> = crate::Pair<'a, T, Atomic>;

#[doc(hidden)]
pub struct Data<T: ?Sized>(NonNull<T>, NonNull<AtomicU32>);
#[doc(hidden)]
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
    /// Borrows the wrapped data.
    ///
    /// This is an alias for [`Lich::borrow`].
    fn borrow(&self) -> &T {
        self.borrow()
    }
}

impl<T: ?Sized> Lich<T> {
    /// Borrows the wrapped data.
    ///
    /// This provides safe, shared access to the underlying data. The borrow is
    /// statically guaranteed to be valid as long as the [`Lich<T, Atomic>`]
    /// exists.
    #[allow(clippy::should_implement_trait)]
    pub fn borrow(&self) -> &T {
        // This is safe because the `Soul`'s drop implementation will block
        // until all `Lich` clones (and therefore all borrows) are gone.
        unsafe { self.0 .0.as_ref() }
    }
}

/// Creates an `atomic` [`Lich<T, Atomic>`] and [`Soul<'a, Atomic>`] pair from a
/// reference and a counter.
///
/// This function binds the lifetime of `value` to a [`Lich<T, Atomic>`] and
/// [`Soul<'a, Atomic>`] pair, using the provided `location` as storage for the
/// reference count.
///
/// The `location` must have a lifetime `'a` that is at least as long as the
/// `value`'s borrow. It will be initialized to `1`.
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

/// Safely consumes an `atomic` [`Lich<T, Atomic>`] and [`Soul<'a, Atomic>`]
/// pair.
///
/// If the provided [`Lich<T, Atomic>`] and [`Soul<'a, Atomic>`] match, they are
/// consumed and `Ok` is returned. If they do not match, `Err` is returned with
/// the pair.
///
/// Unlike the `raw` variant, this function is not strictly required. If the
/// [`Lich<T, Atomic>`] and [`Soul<'a, Atomic>`] are simply dropped, the
/// [`Soul<'a, Atomic>`]'s drop implementation will block until all
/// [`Lich<T, Atomic>`] clones are dropped, ensuring safety. However,
/// using `redeem` is good practice for explicit cleanup.
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
