use crate::{Binding, Sever, TrySever, shroud::Shroud};
use core::{
    marker::PhantomData,
    ptr::{self, NonNull},
};

pub struct Raw;

pub type Soul<'a> = crate::Soul<'a, Raw>;
pub type Lich<T> = crate::Lich<T, Raw>;
pub type Pair<'a, T> = crate::Pair<'a, T, Raw>;

unsafe impl<'a, T: ?Sized + 'a> Send for Data<T> where &'a T: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Data<T> where &'a T: Sync {}

pub struct Data<T: ?Sized>(NonNull<T>);
pub struct Life<'a>(NonNull<()>, PhantomData<&'a ()>);

impl<T: ?Sized> TrySever for Data<T> {
    fn try_sever(&mut self) -> Option<bool> {
        Some(sever_panic())
    }
}

impl Sever for Life<'_> {
    fn sever(&mut self) -> bool {
        sever_panic()
    }
}

impl Binding for Raw {
    type Data<T: ?Sized> = Data<T>;
    type Life<'a> = Life<'a>;

    /// This function can return false positives if the same `&'a T` is bound
    /// twice and the `Self::Data<T>` of the first binding is checked against
    /// the `Self::Life<'a>` of the second.
    fn are_bound<'a, T: ?Sized>(data: &Self::Data<T>, life: &Self::Life<'a>) -> bool {
        ptr::addr_eq(data.0.as_ptr(), life.0.as_ptr())
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
    /// dropped and lives for whole duration of the borrow. Otherwise, this may
    /// result in a *use after free*.
    pub unsafe fn borrow(&self) -> &T {
        unsafe { self.0.0.as_ref() }
    }
}

/// Splits the provided `&'a T` into a [`Lich<S>`] and [`Soul<'a>`] pair that
/// are bound together where `S` is some trait that implements [`Shroud<T>`].
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> Pair<'a, S> {
    let pointer = S::shroud(value);
    (
        crate::Lich(Data(pointer)),
        crate::Soul(Life(pointer.cast(), PhantomData)),
    )
}

/// Safely disposes of a [`Lich<T>`] and a [`Soul<'a>`] that were bound together
/// by a [`ritual`]. Without this call, the [`Lich<T>`] and the [`Soul<'a>`]
/// will panic on drop.
///
/// Contrarily to other [`Bind`]ings this call to [`redeem`] may surprisingly
/// accept a [`Lich<T>`] and a [`Soul<'a>`] that refer to the same `&'a T` but
/// that have not been bound by the same [`ritual`] since the internal check
/// uses a simple address comparison. This will not lead to undefined behavior
/// since the other [`Lich<T>`]es and [`Soul<'a>`]s have a 1 to 1 counterpart
/// and must still each be [`redeem`]ed. This is by design of the zero cost
/// [`Raw`] variant since a more robust mechanism would incur a
/// performance/memory cost.
///
/// Returns `Ok(())` if the [`Lich<T>`] and [`Soul<'a>`] were bound together and
/// [`redeem`]ed, otherwise `Err((lich, soul))`. Note that the [`Lich<T>`] and
/// the [`Soul<'a>`] contained in the error will panic on drop and therefore
/// must be properly [`redeem`]ed.
pub fn redeem<'a, T: ?Sized + 'a>(lich: Lich<T>, soul: Soul<'a>) -> Result<(), Pair<'a, T>> {
    crate::redeem::<_, _, false>(lich, soul).map(|_| {})
}

fn sever_panic() -> bool {
    #[cfg(feature = "std")]
    if std::thread::panicking() {
        return false;
    }

    #[cfg(not(feature = "std"))]
    {
        use core::sync::atomic::{AtomicBool, Ordering};

        static PANIC: AtomicBool = AtomicBool::new(false);
        if PANIC.swap(true, Ordering::Relaxed) {
            return false;
        }
    }

    panic!("this `Raw` order `Lich<T>` must be redeemed")
}
