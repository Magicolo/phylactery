//! Zero-cost, `unsafe` lifetime extension.
//!
//! This module provides the `raw` binding, which is the most performant but
//! also the most dangerous variant. It offers a zero-cost abstraction, meaning
//! it introduces no heap allocations or reference counting overhead. The
//! [`Lich<T, Raw>`] and [`Soul<'a, Raw>`] are simple wrappers around [raw pointers].
//!
//! # Trade-offs
//!
//! - **Pros:**
//!   - Zero-cost; no runtime overhead.
//!   - `#[no_std]` compatible.
//!   - Can be sent to other threads (if `T` is [`Send`] + [`Sync`]).
//! - **Cons:**
//!   - Requires `unsafe` to borrow the data from [`Lich<T, Raw>`].
//!   - [`Lich<T, Raw>`] and [`Soul<'a, Raw>`] **must** be manually `redeem`ed.
//!     Failure to do so will result in a [`panic!`] on drop.
//!   - [`Lich<T, Raw>`] cannot be cloned.
//!
//! # Usage
//!
//! This variant is suitable for performance-critical scenarios where the
//! programmer can manually guarantee the lifetime constraints and the cleanup
//! process.
//!
//! ```
//! use phylactery::{shroud, raw::{ritual, redeem}};
//!
//! pub trait Trait {
//!     fn do_it(&self);
//! }
//! shroud!(Trait);
//!
//! struct Foo(i32);
//! impl Trait for Foo {
//!     fn do_it(&self) {
//!         println!("Value is: {}", self.0);
//!     }
//! }
//!
//! let foo = Foo(42);
//!
//! // Create the Lich/Soul pair.
//! let (lich, soul) = ritual::<_, dyn Trait>(&foo);
//!
//! // Later, in a 'static context...
//! // Safety: We know the `soul` is still in scope, so borrowing is safe.
//! let borrowed_fn = unsafe { lich.borrow() };
//! borrowed_fn.do_it();
//!
//! // The pair must be redeemed to avoid a panic.
//! redeem(lich, soul).ok().unwrap();
//! ```
use crate::{shroud::Shroud, Binding, Sever, TrySever};
use core::{
    marker::PhantomData,
    ptr::{self, NonNull},
};

/// The zero-cost `Binding` variant.
///
/// See the [module-level documentation](self) for more details.
pub struct Raw;

/// A [`Soul<'a, B>`](crate::Soul) bound to the `raw` variant.
pub type Soul<'a> = crate::Soul<'a, Raw>;
/// A [`Lich<T, B>`](crate::Lich) bound to the `raw` variant.
pub type Lich<T> = crate::Lich<T, Raw>;
/// A [`Pair<'a, T, B>`](crate::Pair) bound to the `raw` variant.
pub type Pair<'a, T> = crate::Pair<'a, T, Raw>;

unsafe impl<'a, T: ?Sized + 'a> Send for Data<T> where &'a T: Send {}
unsafe impl<'a, T: ?Sized + 'a> Sync for Data<T> where &'a T: Sync {}

#[doc(hidden)]
pub struct Data<T: ?Sized>(NonNull<T>);
#[doc(hidden)]
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

    /// `Self::Life<'a>` is always bounded until redeemed.
    fn is_life_bound(_: &Self::Life<'_>) -> bool {
        true
    }

    /// `Self::Data<T>` is always bounded until redeemed.
    fn is_data_bound<T: ?Sized>(_: &Self::Data<T>) -> bool {
        true
    }
}

impl<T: ?Sized> Lich<T> {
    /// Borrows the wrapped data.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the corresponding [`Soul<'a, Raw>`] is still
    /// alive and in scope. Dropping the [`Soul<'a, Raw>`] while this borrow is
    /// active will invalidate the pointer, leading to a **use-after-free**
    /// vulnerability.
    ///
    /// The `raw` variant offers no runtime checks to prevent this. It is the
    /// caller's responsibility to uphold this safety contract.
    pub unsafe fn borrow(&self) -> &T {
        unsafe { self.0 .0.as_ref() }
    }
}

/// Creates a `raw` [`Lich<T, Raw>`] and [`Soul<'a, Raw>`] pair from a reference.
///
/// This is a zero-cost operation that creates a [`Lich<T, Raw>`] and
/// [`Soul<'a, Raw>`] by wrapping the provided reference as a raw pointer.
///
/// The returned [`Lich<T, Raw>`] and [`Soul<'a, Raw>`] are intrinsically
/// linked. To prevent a [`panic!`], they **must** be passed to [`redeem`]
/// before the [`Soul<'a, Raw>`]'s lifetime `'a` ends.
pub fn ritual<'a, T: ?Sized + 'a, S: Shroud<T> + ?Sized + 'a>(value: &'a T) -> Pair<'a, S> {
    let pointer = S::shroud(value);
    (
        crate::Lich(Data(pointer)),
        crate::Soul(Life(pointer.cast(), PhantomData)),
    )
}

/// Safely consumes a `raw` [`Lich<T, Raw>`] and [`Soul<'a, Raw>`] pair.
///
/// This function is **required** for the `raw` variant. It safely disposes of
/// the pair, preventing their [`Drop`] implementations from panicking.
///
/// If the provided [`Lich<T, Raw>`] and [`Soul<'a, Raw>`] were created by the
/// same [`ritual`] call, this function will consume them and return `Ok(())`.
/// If they do not match, it will return `Err`, giving the caller ownership of
/// the original pair back.
///
/// # Panics
///
/// The [`Lich<T, Raw>`] and [`Soul<'a, Raw>`] will [`panic!`] on drop if they are
/// not redeemed. It is critical to handle the `Err` case of this function
/// correctly, for example by trying to redeem the pair again with their correct
/// counterparts.
///
/// ```
/// use phylactery::{shroud, raw::{ritual, redeem}};
///
/// pub trait Trait { fn do_it(&self); }
/// shroud!(Trait);
///
/// struct S;
/// impl Trait for S {
///     fn do_it(&self) {}
/// }
///
/// // Create two distinct instances on the stack.
/// let s1 = S;
/// let s2 = S;
///
/// // `s1` and `s2` are guaranteed to have different addresses.
/// let (lich1, soul1) = ritual::<_, dyn Trait>(&s1);
/// let (lich2, soul2) = ritual::<_, dyn Trait>(&s2);
///
/// // This will fail, because the pairs don't match.
/// let err = redeem(lich1, soul2).unwrap_err();
///
/// // The returned pair will panic if dropped, so they must be handled.
/// // We also need to forget the other halves of the original pairs.
/// std::mem::forget(err);
/// std::mem::forget(soul1);
/// std::mem::forget(lich2);
/// ```
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

    panic!("this `Lich<T, Raw>` must be redeemed")
}
