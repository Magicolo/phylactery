use crate::{Bind, Sever, shroud::Shroud};
use core::{
    marker::PhantomData,
    ptr::{self, NonNull},
};
use std::thread;

pub struct Raw;

pub type Soul<'a> = crate::Soul<'a, Raw>;
pub type Lich<T> = crate::Lich<T, Raw>;
pub type Guard<'a, T> = crate::Guard<'a, T, Raw>;
pub type RedeemError<'a, T> = crate::RedeemError<'a, T, Raw>;
pub type RedeemResult<'a, T> = crate::RedeemResult<'a, T, Raw>;

unsafe impl<'a, T: ?Sized + 'a> Send for Lich<T> where &'a T: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Lich<T> where &'a T: Sync {}

pub struct Data<T: ?Sized>(NonNull<T>);
pub struct Life<'a>(NonNull<()>, PhantomData<&'a ()>);

impl<T: ?Sized> Sever for Data<T> {
    fn sever(&mut self) -> bool {
        if thread::panicking() {
            false
        } else {
            panic!("this `Raw` order `Lich<T>` must be redeemed")
        }
    }

    fn try_sever(&mut self) -> Option<bool> {
        None
    }
}

impl Sever for Life<'_> {
    fn sever(&mut self) -> bool {
        if thread::panicking() {
            false
        } else {
            panic!("this `Raw` order `Lich<T>` must be redeemed")
        }
    }

    fn try_sever(&mut self) -> Option<bool> {
        None
    }
}

impl Bind for Raw {
    type Data<T: ?Sized> = Data<T>;
    type Life<'a> = Life<'a>;
    type Refer<'a, T: ?Sized + 'a> = &'a T;

    fn bind<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(
        value: &'a T,
    ) -> (Self::Data<S>, Self::Life<'a>) {
        let pointer = S::shroud(value);
        (Data(pointer), Life(pointer.cast(), PhantomData))
    }

    /// This function can return false positives if the same `&'a T` is bound
    /// twice and the `Self::Data<T>` of the first binding is checked against
    /// the `Self::Life<'a>` of the second.
    fn are_bound<'a, T: ?Sized>(strong: &Self::Data<T>, weak: &Self::Life<'a>) -> bool {
        ptr::addr_eq(strong.0.as_ptr(), weak.0.as_ptr())
    }

    /// `Raw` order liches are always bounded until redeemed.
    fn is_life_bound(_: &Self::Life<'_>) -> bool {
        true
    }

    /// `Raw` order liches are always bounded until redeemed.
    fn is_data_bound<T: ?Sized>(_: &Self::Data<T>) -> bool {
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
pub unsafe fn redeem<'a, T: ?Sized + 'a>(lich: Lich<T>, soul: Soul<'a>) -> RedeemResult<'a, T> {
    // TODO: For a valid `Lich<T>`, this will always return `Ok(Some(sould))` and
    // then panic when the soul is dropped.
    unsafe { crate::redeem(lich, soul) }
}
