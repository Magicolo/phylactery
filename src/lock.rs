use crate::{Order, sever::Sever, shroud::Shroud};
use core::ptr;
use std::sync::{Arc, RwLock, RwLockReadGuard, TryLockError, Weak};

pub struct Lock;

pub type Soul<'a> = crate::Soul<'a, Lock>;
pub type Lich<T> = crate::Lich<T, Lock>;
pub type Guard<'a, T> = crate::Guard<'a, T, Lock>;

impl Order for Lock {
    type Refer<'a, T: ?Sized + 'a> = RwLockReadGuard<'a, Option<*const T>>;
    type Strong<T: ?Sized> = Arc<RwLock<Option<*const T>>>;
    type Weak<'a> = Weak<RwLock<dyn Sever + 'a>>;

    fn bind<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(
        value: &'a T,
    ) -> (Self::Strong<S>, Self::Weak<'a>) {
        let strong = Arc::new(RwLock::new(Some(S::shroud(value))));
        let weak = Arc::downgrade(&strong);
        (strong, weak)
    }

    fn are_bound<'a, T: ?Sized>(strong: &Self::Strong<T>, weak: &Self::Weak<'a>) -> bool {
        ptr::addr_eq(Arc::as_ptr(strong), Weak::as_ptr(weak))
    }

    fn is_bound_weak(weak: &Self::Weak<'_>) -> bool {
        Weak::strong_count(weak) > 0
    }

    fn is_bound_strong<T: ?Sized>(strong: &Self::Strong<T>) -> bool {
        Arc::weak_count(strong) > 0
    }

    fn try_sever_strong<T: ?Sized>(strong: &Self::Strong<T>) -> Option<bool> {
        match strong.try_write() {
            Ok(mut guard) => Some(guard.sever()),
            Err(TryLockError::Poisoned(mut error)) => Some(error.get_mut().sever()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    fn try_sever_weak(weak: &Self::Weak<'_>) -> Option<bool> {
        match weak.upgrade()?.try_write() {
            Ok(mut guard) => Some(guard.sever()),
            Err(TryLockError::Poisoned(mut error)) => Some(error.get_mut().sever()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    fn sever_strong<T: ?Sized>(strong: &Self::Strong<T>) -> bool {
        match strong.write() {
            Ok(mut guard) => guard.sever(),
            Err(mut error) => error.get_mut().sever(),
        }
    }

    fn sever_weak(weak: &Self::Weak<'_>) -> bool {
        match weak.upgrade() {
            Some(strong) => match strong.write() {
                Ok(mut guard) => guard.sever(),
                Err(mut error) => error.get_mut().sever(),
            },
            None => false,
        }
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

pub fn redeem<'a, T: ?Sized + 'a>(lich: Lich<T>, soul: Soul<'a>) -> Option<(Lich<T>, Soul<'a>)> {
    unsafe { crate::redeem(lich, soul) }
}
