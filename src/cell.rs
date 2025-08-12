//! Single-threaded lifetime extension using [`Rc<RefCell<T>>`].
//!
//! This module provides the `cell` binding, which uses [`Rc`] and [`RefCell`]
//! to enable lifetime extension in a single-threaded context. It performs heap
//! allocation for the reference-counted pointer.
//!
//! # Trade-offs
//!
//! - **Pros:**
//!   - Safe, `unsafe`-free public API.
//!   - [`Lich<T, Cell>`] can be cloned.
//!   - `redeem` is not strictly required; dropping is safe.
//!   - Supports `sever` to explicitly break the link.
//! - **Cons:**
//!   - **Not** thread-safe (`!Send` and `!Sync`).
//!   - Allocates on the heap.
//!   - Borrowing from [`Lich<T, Cell>`] returns an [`Option`] and can fail.
//!   - If a borrow is held when the [`Soul<'a, Cell>`] is dropped, the thread
//!     will [`panic!`].
//!
//! # Usage
//!
//! ```
//! use phylactery::{shroud, cell::{ritual, redeem}};
//!
//! pub trait Trait {
//!     fn do_it(&self);
//! }
//! shroud!(Trait);
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
//! let lich_clone = lich.clone();
//!
//! if let Some(f) = lich_clone.borrow() {
//!     f.do_it();
//! }
//!
//! if let Some(f) = lich.borrow() {
//!     f.do_it();
//! }
//!
//! // You can explicitly sever the connection.
//! soul.sever();
//!
//! // Now, borrowing will fail.
//! assert!(lich.borrow().is_none());
//!
//! // `redeem` is not required, but is good practice.
//! // redeem(lich, soul).ok();
//! ```
use crate::{shroud::Shroud, Binding, Sever, TrySever};
use core::{
    cell::{Ref, RefCell},
    ops::Deref,
    ptr::{self, NonNull},
};
use std::rc::{Rc, Weak};

/// The `Rc<RefCell<T>>`-based `Binding` variant.
///
/// See the [module-level documentation](self) for more details.
pub struct Cell;

/// A [`Soul<'a, B>`](crate::Soul) bound to the `cell` variant.
pub type Soul<'a> = crate::Soul<'a, Cell>;
/// A [`Lich<T, B>`](crate::Lich) bound to the `cell` variant.
pub type Lich<T> = crate::Lich<T, Cell>;
/// A [`Pair<'a, T, B>`](crate::Pair) bound to the `cell` variant.
pub type Pair<'a, T> = crate::Pair<'a, T, Cell>;

#[doc(hidden)]
pub struct Data<T: ?Sized>(Rc<RefCell<Option<NonNull<T>>>>);
#[doc(hidden)]
pub struct Life<'a>(Weak<RefCell<dyn Slot + 'a>>);
/// A RAII guard for a borrow from a `cell` [`Lich<T, Cell>`].
///
/// This guard ensures that the borrow from the underlying [`RefCell`] is
/// properly released when the guard is dropped.
///
/// It dereferences to `T`.
pub struct Guard<'a, T: ?Sized>(Ref<'a, Option<NonNull<T>>>);

trait Slot: Sever + TrySever {}
impl<S: Sever + TrySever> Slot for S {}

unsafe impl<'a, T: ?Sized + 'a> Send for Data<T> where Rc<RefCell<Option<&'a T>>>: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Data<T> where Rc<RefCell<Option<&'a T>>>: Sync {}

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
        if Rc::strong_count(&self.0) == 1 {
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

impl Binding for Cell {
    type Data<T: ?Sized> = Data<T>;
    type Life<'a> = Life<'a>;

    fn are_bound<'a, T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'a>) -> bool {
        ptr::addr_eq(Rc::as_ptr(&data.0), Weak::as_ptr(&life.0))
    }

    fn is_life_bound(life: &Self::Life<'_>) -> bool {
        Weak::strong_count(&life.0) > 0
    }

    fn is_data_bound<T: ?Sized>(data: &Self::Data<T>) -> bool {
        Rc::weak_count(&data.0) > 0
    }
}

impl<T: ?Sized> Lich<T> {
    /// Borrows the wrapped data, returning a [`Guard<T>`] if successful.
    ///
    /// This method will return `Some(Guard)` if the data is available and not
    /// already mutably borrowed. The returned [`Guard<T>`] provides immutable
    /// access to the data.
    ///
    /// It will return `None` if:
    /// - The link to the [`Soul<'a, Cell>`] has been severed (e.g.,
    ///   [`Soul::sever`] was called or the [`Soul<'a, Cell>`] was dropped).
    /// - The underlying [`RefCell`] is already mutably borrowed (which can
    ///   happen during `sever` or `redeem`).
    pub fn borrow(&self) -> Option<Guard<'_, T>> {
        // `try_borrow` can be used here because only the `sever` operation calls
        // `borrow_mut`, at which point, the value must not be observable
        let guard = self.0 .0.try_borrow().ok()?;
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

/// Creates a `cell` [`Lich<T, Cell>`] and [`Soul<'a, Cell>`] pair from a
/// reference.
///
/// This function allocates a `Rc<RefCell<...>>` on the heap to manage the
/// reference and its borrow state.
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> Pair<'a, S> {
    let data = Rc::new(RefCell::new(Some(S::shroud(value))));
    let life = Rc::downgrade(&data);
    (crate::Lich(Data(data)), crate::Soul(Life(life)))
}

/// Safely consumes a `cell` [`Lich<T, Cell>`] and [`Soul<'a, Cell>`] pair.
///
/// If the provided [`Lich<T, Cell>`] and [`Soul<'a, Cell>`] match, they are
/// consumed and `Ok` is returned. If they do not match, `Err` is returned with
/// the pair.
///
/// While not strictly necessary for safety (dropping is safe in the `cell`
/// variant), using `redeem` is good practice. It also allows the user to check
/// if the [`Lich<T, Cell>`] was successfully destroyed or if other clones still
/// exist.
///
/// If other [`Lich<T, Cell>`] clones exist, `Ok(Some(soul))` is returned, giving
/// back the [`Soul<'a, Cell>`] to `redeem` the remaining clones later.
pub fn redeem<'a, T: ?Sized + 'a>(
    lich: Lich<T>,
    soul: Soul<'a>,
) -> Result<Option<Soul<'a>>, Pair<'a, T>> {
    crate::redeem::<_, _, true>(lich, soul)
}

fn sever<T: Sever + ?Sized>(cell: &RefCell<T>) -> bool {
    cell.borrow_mut().sever()
}

fn try_sever<T: TrySever + ?Sized>(cell: &RefCell<T>) -> Option<bool> {
    cell.try_borrow_mut().ok()?.try_sever()
}
