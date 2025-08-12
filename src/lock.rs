//! Thread-safe lifetime extension using [`Arc<RwLock<T>>`].
//!
//! This module provides the `lock` binding, which uses [`Arc`] and [`RwLock`]
//! to enable lifetime extension in a thread-safe context. It performs heap
//! allocation for the atomically reference-counted pointer.
//!
//! # Trade-offs
//!
//! - **Pros:**
//!   - Safe, `unsafe`-free public API.
//!   - Thread-safe ([`Send`] and [`Sync`]).
//!   - [`Lich<T, Lock>`] can be cloned and sent across threads.
//!   - `redeem` is not strictly required; dropping is safe.
//!   - Supports `sever` to explicitly break the link.
//! - **Cons:**
//!   - Allocates on the heap.
//!   - Incurs the overhead of [`RwLock`] for borrows.
//!   - Borrowing from [`Lich<T, Lock>`] returns an [`Option`] and can fail.
//!   - If a borrow is held when the [`Soul<'a, Lock>`] is dropped, the thread
//!     will block, which can lead to deadlocks.
//!
//! # Usage
//!
//! ```
//! use phylactery::{shroud, lock::{ritual, redeem}};
//! use std::thread;
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
//! let (lich, soul) = ritual::<_, dyn Trait>(&foo);
//!
//! let lich_clone = lich.clone();
//! thread::spawn(move || {
//!     if let Some(f) = lich_clone.borrow() {
//!         f.do_it();
//!     }
//! }).join().unwrap();
//!
//! if let Some(f) = lich.borrow() {
//!     f.do_it();
//! }
//!
//! // `redeem` is not required, but is good practice.
//! redeem(lich, soul).ok();
//! ```
use crate::{shroud::Shroud, Binding, Sever, TrySever};
use core::{
    ops::Deref,
    ptr::{self, NonNull},
};
use std::sync::{Arc, RwLock, RwLockReadGuard, TryLockError, Weak};

/// The `Arc<RwLock<T>>`-based `Binding` variant.
///
/// See the [module-level documentation](self) for more details.
pub struct Lock;

/// A [`Soul<'a, B>`](crate::Soul) bound to the `lock` variant.
pub type Soul<'a> = crate::Soul<'a, Lock>;
/// A [`Lich<T, B>`](crate::Lich) bound to the `lock` variant.
pub type Lich<T> = crate::Lich<T, Lock>;
/// A [`Pair<'a, T, B>`](crate::Pair) bound to the `lock` variant.
pub type Pair<'a, T> = crate::Pair<'a, T, Lock>;

#[doc(hidden)]
pub struct Data<T: ?Sized>(Arc<RwLock<Option<NonNull<T>>>>);
#[doc(hidden)]
pub struct Life<'a>(Weak<RwLock<dyn Slot + 'a>>);
/// A RAII guard for a borrow from a `lock` [`Lich<T, Lock>`].
///
/// This guard ensures that the read lock from the underlying [`RwLock`] is
/// properly released when the guard is dropped.
///
/// It dereferences to `T`.
pub struct Guard<'a, T: ?Sized>(RwLockReadGuard<'a, Option<NonNull<T>>>);

trait Slot: Sever + TrySever {}
impl<S: Sever + TrySever> Slot for S {}

unsafe impl<'a, T: ?Sized + 'a> Send for Data<T> where Arc<RwLock<Option<&'a T>>>: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Data<T> where Arc<RwLock<Option<&'a T>>>: Sync {}

impl<T: ?Sized> Default for Data<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: ?Sized> Clone for Data<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized> Sever for Data<T> {
    fn sever(&mut self) -> bool {
        sever(&self.0)
    }
}

impl<T: ?Sized> TrySever for Data<T> {
    fn try_sever(&mut self) -> Option<bool> {
        // Only sever if there are no other `Self` clones.
        if Arc::strong_count(&self.0) == 1 {
            try_sever(&self.0)
        } else {
            None
        }
    }
}

impl Sever for Life<'_> {
    fn sever(&mut self) -> bool {
        self.0.upgrade().as_deref().is_some_and(sever)
    }
}

impl TrySever for Life<'_> {
    fn try_sever(&mut self) -> Option<bool> {
        // If the `Weak::upgrade` fails, consider the sever to be a success with
        // `Some(false)`.
        self.0.upgrade().as_deref().map_or(Some(false), try_sever)
    }
}

impl Binding for Lock {
    type Data<T: ?Sized> = Data<T>;
    type Life<'a> = Life<'a>;

    fn are_bound<'a, T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'a>) -> bool {
        ptr::addr_eq(Arc::as_ptr(&data.0), Weak::as_ptr(&life.0))
    }

    fn is_life_bound(life: &Self::Life<'_>) -> bool {
        Weak::strong_count(&life.0) > 0
    }

    fn is_data_bound<T: ?Sized>(data: &Self::Data<T>) -> bool {
        Arc::weak_count(&data.0) > 0
    }
}

impl<T: ?Sized> Lich<T> {
    /// Borrows the wrapped data, returning a [`Guard<T>`] if successful.
    ///
    /// This method will return `Some(Guard)` if the data is available and not
    /// already exclusively locked. The returned `Guard` provides immutable,
    /// thread-safe access to the data.
    ///
    /// It will return `None` if:
    /// - The link to the [`Soul<'a, Lock>`] has been severed (e.g.,
    ///   [`Soul::sever`] was called or the [`Soul<'a, Lock>`] was dropped).
    /// - The underlying [`RwLock`] is already exclusively locked for writing
    ///   (which can happen during `sever` or `redeem`).
    pub fn borrow(&self) -> Option<Guard<'_, T>> {
        // `try_read` can be used here because only the `sever` operation takes a
        // `write` lock, at which point, the value must not be observable
        let guard = self.0 .0.try_read().ok()?;
        if guard.is_some() {
            Some(Guard(guard))
        } else {
            None
        }
    }
}

impl<T: ?Sized> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // # Safety
        // The `Option<NonNull<T>>` can only be `Some` as per the check in
        // `Lich<T>::borrow` and could not have been swapped for `None` since it
        // is protected by its corresponding `RwLockReadGuard` guard.
        unsafe { self.0.as_ref().unwrap_unchecked().as_ref() }
    }
}

impl<T: ?Sized> AsRef<T> for Guard<'_, T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

/// Creates a `lock` [`Lich<T, Lock>`] and [`Soul<'a, Lock>`] pair from a
/// reference.
///
/// This function allocates an `Arc<RwLock<...>>` on the heap to manage the
/// reference and its borrow state in a thread-safe way.
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> Pair<'a, S> {
    let data = Arc::new(RwLock::new(Some(S::shroud(value))));
    let life = Arc::downgrade(&data);
    (crate::Lich(Data(data)), crate::Soul(Life(life)))
}

/// Safely consumes a `lock` [`Lich<T, Lock>`] and [`Soul<'a, Lock>`] pair.
///
/// If the provided [`Lich<T, Lock>`] and [`Soul<'a, Lock>`] match, they are
/// consumed and `Ok` is returned. If they do not match, `Err` is returned with
/// the pair.
///
/// While not strictly necessary for safety (dropping is safe in the `lock`
/// variant), using `redeem` is good practice. It also allows the user to check
/// if the [`Lich<T, Lock>`] was successfully destroyed or if other clones still
/// exist.
///
/// If other [`Lich<T, Lock>`] clones exist, `Ok(Some(soul))` is returned, giving
/// back the [`Soul<'a, Lock>`] to `redeem` the remaining clones later.
pub fn redeem<'a, T: ?Sized + 'a>(
    lich: Lich<T>,
    soul: Soul<'a>,
) -> Result<Option<Soul<'a>>, Pair<'a, T>> {
    crate::redeem::<_, _, true>(lich, soul)
}

fn sever<T: Sever + ?Sized>(lock: &RwLock<T>) -> bool {
    match lock.write() {
        Ok(mut guard) => guard.sever(),
        Err(mut error) => error.get_mut().sever(),
    }
}

fn try_sever<T: TrySever + ?Sized>(lock: &RwLock<T>) -> Option<bool> {
    match lock.try_write() {
        Ok(mut guard) => guard.try_sever(),
        Err(TryLockError::Poisoned(mut error)) => error.get_mut().try_sever(),
        Err(TryLockError::WouldBlock) => None,
    }
}
