use core::{
    borrow::Borrow,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{AtomicU32, Ordering},
};

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
/// on a pinned [`Soul`](crate::soul::Soul). It can be cloned and be sent across
/// threads. It dereferences to the value owned by the `Soul`.
///
/// # Safety
///
/// The core safety mechanism of this type is enforced by the
/// [`Soul`](crate::soul::Soul)'s `Drop` implementation. If you attempt to drop
/// a [`Soul`](crate::soul::Soul) while one or more of its [`Lich`]es are still
/// in existence, the `Soul`'s drop will either block the current thread until
/// all [`Lich`]es are dropped. This behavior guarantees that a [`Lich`] can
/// never become a dangling pointer to the [`Soul`](crate::soul::Soul)'s data.
pub struct Lich<T: ?Sized> {
    pub(crate) value: NonNull<T>,
    pub(crate) count: NonNull<AtomicU32>,
}

unsafe impl<T: ?Sized> Send for Lich<T> where for<'a> &'a T: Send {}
unsafe impl<T: ?Sized> Sync for Lich<T> where for<'a> &'a T: Sync {}

impl<T: ?Sized> Lich<T> {
    /// Returns the number of `Lich`es that are currently bound to the
    /// [`Soul`](crate::soul::Soul).
    pub fn bindings(&self) -> usize {
        self.count_ref()
            .load(Ordering::Relaxed)
            .wrapping_add(1)
            .saturating_sub(1) as _
    }

    fn count_ref(&self) -> &AtomicU32 {
        // Safety: the pointers are valid for the lifetime of `self`; guaranteed by the
        // reference count.
        unsafe { self.count.as_ref() }
    }

    fn data_ref(&self) -> Result<&T, &'static str> {
        // Safety: the pointers are valid for the lifetime of `self`; guaranteed by the
        // reference count.
        Ok(unsafe { self.value.as_ref() })
    }
}

impl<T: ?Sized> Clone for Lich<T> {
    fn clone(&self) -> Self {
        self.count_ref().fetch_add(1, Ordering::Relaxed);
        Self {
            value: self.value,
            count: self.count,
        }
    }
}

impl<T: ?Sized> Borrow<T> for Lich<T> {
    fn borrow(&self) -> &T {
        self.data_ref().unwrap()
    }
}

impl<T: ?Sized> Deref for Lich<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data_ref().unwrap()
    }
}

impl<T: ?Sized> AsRef<T> for Lich<T> {
    fn as_ref(&self) -> &T {
        self.data_ref().unwrap()
    }
}

impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        let count = self.count_ref();
        match count.fetch_sub(1, Ordering::Relaxed) {
            0 | u32::MAX => unreachable!(),
            1 => atomic_wait::wake_one(count),
            _ => {}
        }
    }
}
