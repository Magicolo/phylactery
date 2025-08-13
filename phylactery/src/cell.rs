//! `Rc<RefCell<T>>`-based, [`Clone`]able, lifetime extension using a reference
//! counter.
//!
//! This module provides the [`Cell`] [`Binding`] implementation, which uses a
//! [`Rc<RefCell<T>>`] as a reference counter to track the number of active
//! [`Lich<T>`] clones/borrows.

use crate::{Binding, Sever, TrySever, shroud::Shroud};
use core::{
    cell::{Ref, RefCell},
    ops::Deref,
    ptr::{self, NonNull},
};
use std::rc::{Rc, Weak};

pub struct Cell;
pub type Soul<'a> = crate::Soul<'a, Cell>;
pub type Lich<T> = crate::Lich<T, Cell>;
pub type Pair<'a, T> = crate::Pair<'a, T, Cell>;
pub struct Data<T: ?Sized>(Rc<RefCell<Option<NonNull<T>>>>);
pub struct Life<'a>(Weak<RefCell<dyn Slot + 'a>>);
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
    /// This method will return [`Some<Guard>`] if the data is available and not
    /// already mutably borrowed. The returned [`Guard<T>`] provides immutable
    /// access to the data.
    ///
    /// It will return [`None`] if:
    /// - The link to the [`Soul<'a>`] has been severed (e.g., [`Soul::sever`]
    ///   was called or the [`Soul<'a>`] was dropped).
    /// - The underlying [`RefCell`] is already mutably borrowed (which can
    ///   happen during [`Sever::sever`] or [`redeem`]).
    pub fn borrow(&self) -> Option<Guard<'_, T>> {
        // `try_borrow` can be used here because only the `sever` operation calls
        // `borrow_mut`, at which point, the value must not be observable
        let guard = self.0.0.try_borrow().ok()?;
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

/// Binds the lifetime of `value` to a [`Lich<T>`] and [`Soul<'a>`] pair.
///
/// This function allocates a [`Rc<RefCell<..>>`] on the heap to manage the
/// reference.
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> Pair<'a, S> {
    let data = Rc::new(RefCell::new(Some(S::shroud(value))));
    let life = Rc::downgrade(&data);
    (crate::Lich(Data(data)), crate::Soul(Life(life)))
}

/// Safely disposes of a [`Lich<T>`] and [`Soul<'a>`] pair.
///
/// If the provided [`Lich<T>`] and [`Soul<'a>`] are bound together, they are
/// consumed and [`Ok`] is returned with the [`Soul<'a>`] if there are other
/// live [`Lich<T>`] clones. If they are not bound together, [`Err`] is returned
/// with the pair.
///
/// If the [`Lich<T>`] and [`Soul<'a>`] are simply dropped, the [`Soul<'a>`]'s
/// [`Drop`] implementation will [`panic`] if any remaining [`Lich<T>::borrow`]
/// [`Guard`]s are still alive, ensuring safety. While not strictly necessary,
/// using [`redeem`] is good practice for explicit cleanup.
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
