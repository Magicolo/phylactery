use crate::{Bind, Sever, shroud::Shroud};
use core::ptr::{self, NonNull};
use std::sync::{Arc, RwLock, RwLockReadGuard, TryLockError, Weak};

pub struct Lock;

pub type Soul<'a> = crate::Soul<'a, Lock>;
pub type Lich<T> = crate::Lich<T, Lock>;
pub type Guard<'a, T> = crate::Guard<'a, T, Lock>;
pub type RedeemError<'a, T> = crate::RedeemError<'a, T, Lock>;
pub type RedeemResult<'a, T> = crate::RedeemResult<'a, T, Lock>;

unsafe impl<'a, T: ?Sized + 'a> Send for Lich<T> where Arc<RwLock<Option<&'a T>>>: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Lich<T> where Arc<RwLock<Option<&'a T>>>: Sync {}

impl<T: Sever + ?Sized> Sever for Arc<RwLock<T>> {
    fn sever(&mut self) -> bool {
        match self.write() {
            Ok(mut guard) => guard.sever(),
            Err(mut error) => error.get_mut().sever(),
        }
    }

    fn try_sever(&mut self) -> Option<bool> {
        match self.try_write() {
            Ok(mut guard) => guard.try_sever(),
            Err(TryLockError::Poisoned(mut error)) => error.get_mut().try_sever(),
            Err(TryLockError::WouldBlock) => None,
        }
    }
}

impl<T: Sever + ?Sized> Sever for Weak<RwLock<T>> {
    fn sever(&mut self) -> bool {
        self.upgrade().is_some_and(|mut strong| strong.sever())
    }

    fn try_sever(&mut self) -> Option<bool> {
        self.upgrade()
            .as_mut()
            .map_or(Some(false), Sever::try_sever)
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

pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> (Lich<S>, Soul<'a>) {
    crate::ritual(value)
}

pub fn redeem<'a, T: ?Sized + 'a>(lich: Lich<T>, soul: Soul<'a>) -> RedeemResult<'a, T> {
    unsafe { crate::redeem(lich, soul, true) }
}
