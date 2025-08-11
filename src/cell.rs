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
        // `Lich<T>::borrow` and could not have been swapped for `None` since it is
        // protected by its corresponding `RwLockReadGuard` guard.
        unsafe { self.0.as_ref().unwrap_unchecked().as_ref() }
    }
}

impl<T: ?Sized> AsRef<T> for Guard<'_, T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

/// Splits the provided `&'a T` into a [`Lich<S>`] and [`Soul<'a>`] pair that
/// are bound together where `S` is some trait that implements [`Shroud<T>`].
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> Pair<'a, S> {
    let data = Rc::new(RefCell::new(Some(S::shroud(value))));
    let life = Rc::downgrade(&data);
    (crate::Lich(Data(data)), crate::Soul(Life(life)))
}

/// Disposes of a [`Lich<T>`] and a [`Soul<'a>`] that were bound together by the
/// same [`ritual`].
///
/// While it is not strictly necessary for the [`Cell`] variant to use this call
/// since is safe to simply let the [`Lich<T>`] or the [`Soul<'a>`] be dropped,
/// it is considered good practice to ensure consistent usage across all
/// variants and to convince oneself that no borrow remain alive.
///
/// Returns `Ok(..)` if the [`Lich<T>`] and [`Soul<'a>`] were bound by the same
/// [`ritual`] and [`redeem`]ed. The [`Soul<'a>`] will be returned if more
/// instances of [`Lich<T>`] remain to allow them to be [`redeem`]ed. Otherwise
/// returns `Err((lich, soul))` such that they can be properly [`redeem`]ed.
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
