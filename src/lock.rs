use crate::{Bind, Sever, shroud::Shroud};
use core::ptr::{self, NonNull};
use std::sync::{Arc, RwLock, RwLockReadGuard, TryLockError, Weak};

pub struct Lock;

pub type Soul<'a> = crate::Soul<'a, Lock>;
pub type Lich<T> = crate::Lich<T, Lock>;
pub type Guard<'a, T> = crate::Guard<'a, T, Lock>;

unsafe impl<'a, T: ?Sized + 'a> Send for Lich<T> where Arc<RwLock<Option<&'a T>>>: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Lich<T> where Arc<RwLock<Option<&'a T>>>: Sync {}

impl<T: Sever + ?Sized> Sever for Arc<RwLock<T>> {
    fn sever(&mut self) -> bool {
        sever(self)
    }

    fn try_sever(&mut self) -> Option<bool> {
        // Only sever if there are no other `Self` clones.
        if Arc::strong_count(self) == 1 {
            try_sever(self)
        } else {
            None
        }
    }
}

impl<T: Sever + ?Sized> Sever for Weak<RwLock<T>> {
    fn sever(&mut self) -> bool {
        self.upgrade().as_deref().is_some_and(sever)
    }

    fn try_sever(&mut self) -> Option<bool> {
        // If the `Weak::upgrade` fails, consider the sever to be a success with
        // `Some(false)`.
        self.upgrade().as_deref().map_or(Some(false), try_sever)
    }
}

impl Bind for Lock {
    type Data<T: ?Sized> = Arc<RwLock<Option<NonNull<T>>>>;
    type Life<'a> = Weak<RwLock<dyn Sever + 'a>>;
    type Refer<'a, T: ?Sized + 'a> = RwLockReadGuard<'a, Option<NonNull<T>>>;

    fn bind<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(
        value: &'a T,
    ) -> (Self::Data<S>, Self::Life<'a>) {
        let data = Arc::new(RwLock::new(Some(S::shroud(value))));
        let life = Arc::downgrade(&data);
        (data, life)
    }

    fn are_bound<'a, T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'a>) -> bool {
        ptr::addr_eq(Arc::as_ptr(data), Weak::as_ptr(life))
    }

    fn is_life_bound(life: &Self::Life<'_>) -> bool {
        Weak::strong_count(life) > 0
    }

    fn is_data_bound<T: ?Sized>(data: &Self::Data<T>) -> bool {
        Arc::weak_count(data) > 0
    }
}

impl<T: ?Sized> Lich<T> {
    pub fn borrow(&self) -> Option<Guard<'_, T>> {
        // `try_read` can be used here because only the `sever` operation takes a
        // `write` lock, at which point, the value must not be observable
        let guard = self.0.try_read().ok()?;
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
    crate::redeem::<_, _, true>(lich, soul)
}

fn sever<T: Sever + ?Sized>(lock: &RwLock<T>) -> bool {
    match lock.write() {
        Ok(mut guard) => guard.sever(),
        Err(mut error) => error.get_mut().sever(),
    }
}

fn try_sever<T: Sever + ?Sized>(lock: &RwLock<T>) -> Option<bool> {
    match lock.try_write() {
        Ok(mut guard) => guard.try_sever(),
        Err(TryLockError::Poisoned(mut error)) => error.get_mut().try_sever(),
        Err(TryLockError::WouldBlock) => None,
    }
}
