use crate::Binding;
use core::{borrow::Borrow, ops::Deref, ptr::NonNull};

/// A `'static` pointer to a value owned by a [`Soul`](crate::soul::Soul).
///
/// A [`Lich`] acts like a `&'static T`, but its validity is dynamically tied to
/// the lifetime of its parent [`Soul`](crate::soul::Soul) rather than being
/// statically determined by the Rust compiler. This allows for safely
/// referencing stack-local or other non-`'static` data in contexts that require
/// a `'static` lifetime, such as newly spawned threads.
///
/// # Usage
///
/// A [`Lich`] is created by calling [`Soul::bind()`](crate::soul::Soul::bind)
/// on a pinned [`Soul`](crate::soul::Soul). It can be cloned freely and,
/// depending on the [`Binding`] used, may be sent across threads. It
/// dereferences to the value owned by the `Soul`.
///
/// # Safety
///
/// The core safety mechanism of this type is enforced by the
/// [`Soul`](crate::soul::Soul)'s `Drop` implementation. If you attempt to drop
/// a [`Soul`](crate::soul::Soul) while one or more of its [`Lich`]es are still
/// in existence, the `Soul`'s drop will either block the current thread until
/// all [`Lich`]es are dropped, or it will panic. This behavior depends on the
/// chosen [`Binding`] and guarantees that a [`Lich`] can never become a
/// dangling pointer to the [`Soul`](crate::soul::Soul)'s data.
pub struct Lich<T: ?Sized, B: Binding + ?Sized> {
    pub(crate) value: NonNull<T>,
    pub(crate) bind: NonNull<B>,
}

unsafe impl<T: ?Sized, B: Binding + ?Sized> Send for Lich<T, B>
where
    for<'a> &'a T: Send,
    for<'a> &'a B: Send,
{
}
unsafe impl<T: ?Sized, B: Binding + ?Sized> Sync for Lich<T, B>
where
    for<'a> &'a T: Sync,
    for<'a> &'a B: Sync,
{
}

impl<T: ?Sized, B: Binding + ?Sized> Lich<T, B> {
    /// Returns the number of `Lich`es that are currently bound to the
    /// [`Soul`](crate::soul::Soul).
    pub fn bindings(&self) -> usize {
        self.bind_ref().count() as _
    }

    const fn bind_ref(&self) -> &B {
        // Safety: the pointers are valid for the lifetime of `self`; guaranteed by the
        // `B: Binding`'s reference count.
        unsafe { self.bind.as_ref() }
    }

    const fn data_ref(&self) -> &T {
        // Safety: the pointers are valid for the lifetime of `self`; guaranteed by the
        // `B: Binding`'s reference count.
        unsafe { self.value.as_ref() }
    }
}

impl<T: ?Sized, B: Binding + ?Sized> Clone for Lich<T, B> {
    fn clone(&self) -> Self {
        self.bind_ref().increment();
        Self {
            value: self.value,
            bind: self.bind,
        }
    }
}

impl<T: ?Sized, B: Binding + ?Sized> Borrow<T> for Lich<T, B> {
    fn borrow(&self) -> &T {
        self.data_ref()
    }
}

impl<T: ?Sized, B: Binding + ?Sized> Deref for Lich<T, B> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data_ref()
    }
}

impl<T: ?Sized, B: Binding + ?Sized> AsRef<T> for Lich<T, B> {
    fn as_ref(&self) -> &T {
        self.data_ref()
    }
}

impl<T: ?Sized, B: Binding + ?Sized> Drop for Lich<T, B> {
    fn drop(&mut self) {
        if B::bail() {
            return;
        }

        let bind = self.bind_ref();
        match bind.decrement() {
            0 | u32::MAX => unreachable!(),
            1 => bind.redeem(),
            _ => {}
        }
    }
}
