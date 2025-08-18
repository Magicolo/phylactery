//! [`Rc<RefCell<T>>`]-based, [`Clone`]able, lifetime extension using a
//! reference counter.
//!
//! This module provides the [`Cell`] [`Binding`] implementation, which uses an
//! [`Rc<RefCell<T>>`] as a reference counter to track the number of active
//! [`Lich<T>`] clones/borrows.

use crate::{Sever, TrySever, shroud::Shroud};
use core::{
    cell::{Ref, RefCell},
    mem::ManuallyDrop,
    ops::Deref,
    ptr::{self, NonNull, drop_in_place},
};
use std::rc::{Rc, Weak};

pub struct Lich<T: ?Sized>(Rc<RefCell<Option<NonNull<T>>>>);
pub struct Soul<'a>(Weak<RefCell<dyn Slot + 'a>>);
pub struct Guard<'a, T: ?Sized>(Ref<'a, Option<NonNull<T>>>);
pub type Pair<'a, T> = (Lich<T>, Soul<'a>);

trait Slot: Sever + TrySever {}
impl<S: Sever + TrySever> Slot for S {}

unsafe impl<'a, T: ?Sized + 'a> Send for Lich<T> where Rc<RefCell<Option<&'a T>>>: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Lich<T> where Rc<RefCell<Option<&'a T>>>: Sync {}

impl<T: ?Sized> Default for Lich<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: ?Sized> Clone for Lich<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized> Lich<T> {
    /// Borrows the wrapped data, returning a [`Guard<T>`] if successful.
    ///
    /// This method will return a [`Some<Guard>`] if the data is available and
    /// not already mutably borrowed. The returned [`Guard<T>`] provides
    /// immutable access to the data.
    ///
    /// It will return [`None`] if:
    /// - The link to the [`Soul<'a>`] has been severed (e.g., [`Soul::sever`]
    ///   was called or the [`Soul<'a>`] was dropped).
    /// - The underlying [`RefCell`] is already mutably borrowed (which can
    ///   happen during [`Sever::sever`] or [`redeem`]).
    pub fn borrow(&self) -> Option<Guard<'_, T>> {
        // `try_borrow` can be used here because only the `sever` operation calls
        // `borrow_mut`, at which point, the value must not be observable
        let guard = self.0.try_borrow().ok()?;
        if guard.is_some() {
            Some(Guard(guard))
        } else {
            None
        }
    }

    pub fn is_bound(&self) -> bool {
        Rc::weak_count(&self.0) > 0
    }

    pub fn sever(self) -> bool {
        sever(&self.0)
    }

    pub fn try_sever(self) -> Result<bool, Self> {
        // Only sever if there are no other `Self` clones.
        if Rc::strong_count(&self.0) == 1 {
            try_sever(&self.0).ok_or(self)
        } else {
            Err(self)
        }
    }
}

impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        try_sever(&self.0);
    }
}

impl Soul<'_> {
    pub fn is_bound(&self) -> bool {
        Weak::strong_count(&self.0) > 0
    }

    pub fn sever(self) -> bool {
        self.0.upgrade().as_deref().is_some_and(sever)
    }

    pub fn try_sever(self) -> Result<bool, Self> {
        self.0
            .upgrade()
            .as_deref()
            .map_or(Some(false), try_sever)
            .ok_or(self)
    }
}

impl Drop for Soul<'_> {
    fn drop(&mut self) {
        if let Some(slot) = self.0.upgrade().as_deref() {
            sever(slot);
        }
    }
}

impl<T: ?Sized> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // # Safety
        // The `Option<NonNull<T>>` can only be `Some` as per the check in
        // `Lich<T>::borrow` and could not have been swapped for `None` since it
        // is protected by its corresponding `Ref` guard.
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
/// This function allocates a [`Rc<RefCell<T>>`] on the heap to manage the
/// reference.
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> Pair<'a, S> {
    let data = Rc::new(RefCell::new(Some(S::shroud(value))));
    let life = Rc::downgrade(&data);
    (Lich(data), Soul(life))
}

/// Safely disposes of a [`Lich<T>`] and [`Soul<'a>`] pair.
///
/// If the provided [`Lich<T>`] and [`Soul<'a>`] are bound together, they are
/// consumed and [`Ok`] is returned with the [`Soul<'a>`] if there are other
/// live [`Lich<T>`] clones. If they are not bound together, [`Err`] is
/// returned with the pair.
///
/// If the [`Lich<T>`] and [`Soul<'a>`] are simply dropped, the [`Soul<'a>`]'s
/// [`Drop`] implementation will [`panic!`] if any remaining
/// [`Lich<T>::borrow`] [`Guard`]s are still alive, ensuring safety. While not
/// strictly necessary, using [`redeem`] is good practice for explicit cleanup.
pub fn redeem<'a, T: ?Sized + 'a>(
    lich: Lich<T>,
    soul: Soul<'a>,
) -> Result<Option<Soul<'a>>, Pair<'a, T>> {
    if ptr::addr_eq(Rc::as_ptr(&lich.0), Weak::as_ptr(&soul.0)) {
        let mut lich = ManuallyDrop::new(lich);
        unsafe { drop_in_place(&mut lich.0) };
        if soul.is_bound() {
            Ok(Some(soul))
        } else {
            let mut soul = ManuallyDrop::new(soul);
            unsafe { drop_in_place(&mut soul.0) };
            Ok(None)
        }
    } else {
        Err((lich, soul))
    }
}

fn sever<T: Sever + ?Sized>(cell: &RefCell<T>) -> bool {
    cell.borrow_mut().sever()
}

fn try_sever<T: TrySever + ?Sized>(cell: &RefCell<T>) -> Option<bool> {
    cell.try_borrow_mut().ok()?.try_sever()
}
