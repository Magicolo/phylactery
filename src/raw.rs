use crate::{Bind, Sever, shroud::Shroud};
use core::{
    marker::PhantomData,
    ptr::{self, NonNull},
};

pub struct Raw;

pub type Soul<'a> = crate::Soul<'a, Raw>;
pub type Lich<T> = crate::Lich<T, Raw>;
pub type Guard<'a, T> = crate::Guard<'a, T, Raw>;

unsafe impl<'a, T: ?Sized + 'a> Send for Lich<T> where &'a T: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Lich<T> where &'a T: Sync {}

pub struct Strong<T: ?Sized>(NonNull<T>);
pub struct Weak<'a>(NonNull<()>, PhantomData<&'a ()>);

impl<T: ?Sized> Sever for Strong<T> {
    fn sever(&mut self) -> bool {
        panic!("this `Raw` order `Lich<T>` must be redeemed")
    }

    fn try_sever(&mut self) -> Option<bool> {
        None
    }
}

impl Sever for Weak<'_> {
    fn sever(&mut self) -> bool {
        panic!("this `Raw` order `Lich<T>` must be redeemed")
    }

    fn try_sever(&mut self) -> Option<bool> {
        None
    }
}

impl Bind for Raw {
    type Refer<'a, T: ?Sized + 'a> = &'a T;
    type Strong<T: ?Sized> = Strong<T>;
    type Weak<'a> = Weak<'a>;

    fn bind<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(
        value: &'a T,
    ) -> (Self::Strong<S>, Self::Weak<'a>) {
        let pointer = S::shroud(value);
        (Strong(pointer), Weak(pointer.cast(), PhantomData))
    }

    /// This function can return false positives if the same `&'a T` is bound
    /// twice and the `Self::Strong<T>` of the first binding is checked against
    /// the `Self::Weak<'a>` of the second.
    fn are_bound<'a, T: ?Sized>(strong: &Self::Strong<T>, weak: &Self::Weak<'a>) -> bool {
        ptr::addr_eq(strong.0.as_ptr(), weak.0.as_ptr())
    }

    /// `Raw` order liches are always bounded until redeemed.
    fn is_bound_weak(_: &Self::Weak<'_>) -> bool {
        true
    }

    /// `Raw` order liches are always bounded until redeemed.
    fn is_bound_strong<T: ?Sized>(_: &Self::Strong<T>) -> bool {
        true
    }
}

impl<T: ?Sized> Lich<T> {
    /// # Safety
    /// The caller must ensure that the associated [`Soul<'a>`] has not been
    /// dropped otherwise, this is undefined behavior.
    pub unsafe fn borrow(&self) -> &T {
        unsafe { self.0.0.as_ref() }
    }
}

pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> (Lich<S>, Soul<'a>) {
    crate::ritual(value)
}

/// # Safety
/// The caller must ensure that the provided [`Lich<T>`] and [`Soul<'a>`] have
/// been created from the same [`ritual`]. The call to [`redeem`] will at least
/// do a pointer comparison to validate whether the two are bound together but
/// since this validation can not be guaranteed without incurring additional
/// performance/memory costs, the burden is shifted to the caller.
pub unsafe fn redeem<'a, T: ?Sized + 'a>(
    lich: Lich<T>,
    soul: Soul<'a>,
) -> Option<(Lich<T>, Soul<'a>)> {
    unsafe { crate::redeem(lich, soul) }
}
