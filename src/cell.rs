use crate::{Bind, Sever, shroud::Shroud};
use core::{
    cell::{Ref, RefCell},
    ptr::{self, NonNull},
};
use std::rc::{Rc, Weak};

pub struct Cell;

pub type Soul<'a> = crate::Soul<'a, Cell>;
pub type Lich<T> = crate::Lich<T, Cell>;
pub type Guard<'a, T> = crate::Guard<'a, T, Cell>;

unsafe impl<'a, T: ?Sized + 'a> Send for Lich<T> where Rc<RefCell<Option<&'a T>>>: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Lich<T> where Rc<RefCell<Option<&'a T>>>: Sync {}

impl<T: Sever + ?Sized> Sever for Rc<RefCell<T>> {
    fn sever(&mut self) -> bool {
        self.borrow_mut().sever()
    }

    fn try_sever(&mut self) -> Option<bool> {
        self.try_borrow_mut().ok()?.try_sever()
    }
}

impl<T: Sever + ?Sized> Sever for Weak<RefCell<T>> {
    fn sever(&mut self) -> bool {
        self.upgrade().is_some_and(|mut strong| strong.sever())
    }

    fn try_sever(&mut self) -> Option<bool> {
        self.upgrade()
            .as_mut()
            .map_or(Some(false), Sever::try_sever)
    }
}

impl Bind for Cell {
    type Data<T: ?Sized> = Rc<RefCell<Option<NonNull<T>>>>;
    type Life<'a> = Weak<RefCell<dyn Sever + 'a>>;
    type Refer<'a, T: ?Sized + 'a> = Ref<'a, Option<NonNull<T>>>;

    fn bind<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(
        value: &'a T,
    ) -> (Self::Data<S>, Self::Life<'a>) {
        let data = Rc::new(RefCell::new(Some(S::shroud(value))));
        let life = Rc::downgrade(&data);
        (data, life)
    }

    fn are_bound<'a, T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'a>) -> bool {
        ptr::addr_eq(Rc::as_ptr(data), Weak::as_ptr(life))
    }

    fn is_life_bound(life: &Self::Life<'_>) -> bool {
        Weak::strong_count(life) > 0
    }

    fn is_data_bound<T: ?Sized>(data: &Self::Data<T>) -> bool {
        Rc::weak_count(data) > 0
    }
}

impl<T: ?Sized> Lich<T> {
    pub fn borrow(&self) -> Option<Guard<'_, T>> {
        // `try_borrow` can be used here because only the `sever` operation calls
        // `borrow_mut`, at which point, the value must not be observable
        let guard = self.0.try_borrow().ok()?;
        if guard.is_some() {
            Some(crate::Guard(guard))
        } else {
            None
        }
    }
}

/// Splits the provided `&'a T` into a [`Lich<S>`] and [`Soul<'a>`] pair that
/// are bound together where `S` is some trait that implements [`Shroud<T>`].
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> (Lich<S>, Soul<'a>) {
    crate::ritual(value)
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
) -> Result<Option<Soul<'a>>, (Lich<T>, Soul<'a>)> {
    crate::redeem(lich, soul, true)
}
