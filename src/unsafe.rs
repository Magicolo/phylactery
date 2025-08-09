use crate::{Order, shroud::Shroud};
use core::ptr;

pub struct Unsafe;

pub type Soul<'a> = crate::Soul<'a, Unsafe>;
pub type Lich<T> = crate::Lich<T, Unsafe>;
pub type Guard<'a, T> = crate::Guard<'a, T, Unsafe>;

impl Order for Unsafe {
    type Refer<'a, T: ?Sized + 'a> = &'a T;
    type Strong<T: ?Sized> = *const T;
    type Weak<'a> = &'a ();

    fn bind<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(
        value: &'a T,
    ) -> (Self::Strong<S>, Self::Weak<'a>) {
        (S::shroud(value), &())
    }

    fn are_bound<'a, T: ?Sized>(strong: &Self::Strong<T>, weak: &Self::Weak<'a>) -> bool {
        ptr::addr_eq(*strong, *weak)
    }

    fn is_bound_weak(_: &Self::Weak<'_>) -> bool {
        true
    }

    fn is_bound_strong<T: ?Sized>(_: &Self::Strong<T>) -> bool {
        true
    }

    fn try_sever_strong<T: ?Sized>(_: &Self::Strong<T>) -> Option<bool> {
        None
    }

    fn try_sever_weak(_: &Self::Weak<'_>) -> Option<bool> {
        None
    }

    fn sever_strong<T: ?Sized>(_: &Self::Strong<T>) -> bool {
        panic!("`Unsafe` kind must be `redeem`ed, not severed");
    }

    fn sever_weak(_: &Self::Weak<'_>) -> bool {
        panic!("`Unsafe` kind must be `redeem`ed, not severed");
    }
}

impl<T: ?Sized> Lich<T> {
    /// # Safety
    /// The caller must ensure that the associated [`Soul<'a>`] has not been
    /// dropped otherwise, this is undefined behavior.
    pub unsafe fn borrow(&self) -> &T {
        unsafe { &*self.0 }
    }
}
