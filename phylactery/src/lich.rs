use crate::soul::SEVERED;
use core::{
    borrow::Borrow,
    fmt,
    mem::forget,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{AtomicU32, Ordering},
};

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
/// [`Soul`](crate::soul::Soul)'s [`Drop`] implementation. If you attempt to
/// drop a [`Soul`](crate::soul::Soul) while one or more of its [`Lich`]es are
/// still in existence, the [`Soul`](crate::soul::Soul)'s drop will either block
/// the current thread until all [`Lich`]es are dropped. This behavior
/// guarantees that a [`Lich`] can never become a dangling pointer to the
/// [`Soul`](crate::soul::Soul)'s data.
pub struct Lich<T: ?Sized> {
    pub(crate) value: NonNull<T>,
    pub(crate) count: NonNull<AtomicU32>,
}

unsafe impl<T: ?Sized> Send for Lich<T> where for<'a> &'a T: Send {}
unsafe impl<T: ?Sized> Sync for Lich<T> where for<'a> &'a T: Sync {}

impl<T: ?Sized> Lich<T> {
    /// Returns the number of `Lich`es that are currently bound to the
    /// [`Soul`](crate::soul::Soul).
    ///
    /// Returns `0` both when no Liches are bound and when the
    /// [`Soul`](crate::soul::Soul) has already been severed.
    #[must_use]
    pub fn bindings(&self) -> usize {
        let raw = self.count_ref().load(Ordering::Relaxed);
        // `SEVERED` (`u32::MAX`) is the severed sentinel; treat it as 0 live bindings.
        raw.wrapping_add(1).saturating_sub(1) as _
    }

    /// Disposes of this [`Lich`], decrementing the binding count for its
    /// parent [`Soul`](crate::soul::Soul).
    ///
    /// This is equivalent to dropping the [`Lich`] but explicitly returns the
    /// remaining number of live [`Lich`]es. Any thread that is blocked in
    /// [`Soul::sever`](crate::soul::Soul::sever) or dropping the
    /// [`Soul`](crate::soul::Soul) waiting for the count to reach zero
    /// will be woken if this was the last [`Lich`].
    ///
    /// Returns the number of [`Lich`]es still bound to the
    /// [`Soul`](crate::soul::Soul) after this one is redeemed.
    pub fn redeem(self) -> usize {
        // Safety: this `Lich` is no longer externally reachable and is
        // `forget(self)` to prevent `drop` from double redeeming.
        let count = unsafe { self.redeem_unchecked() };
        forget(self);
        count
    }

    /// Safety: must be called only once for this `Lich` when it became
    /// unreachable.
    unsafe fn redeem_unchecked(&self) -> usize {
        let count = self.count_ref();
        let remain = decrement(count);
        if remain == 0 {
            atomic_wait::wake_all(count);
        }
        remain as _
    }

    fn count_ref(&self) -> &AtomicU32 {
        // Safety: the pointers are valid for the lifetime of `self`; guaranteed by the
        // reference count.
        unsafe { self.count.as_ref() }
    }

    fn data_ref(&self) -> &T {
        // Safety: the pointers are valid for the lifetime of `self`; guaranteed by the
        // reference count.
        unsafe { self.value.as_ref() }
    }
}

impl<T: ?Sized> Clone for Lich<T> {
    fn clone(&self) -> Self {
        increment(self.count_ref());
        Self {
            value: self.value,
            count: self.count,
        }
    }
}

impl<T: ?Sized> Borrow<T> for Lich<T> {
    fn borrow(&self) -> &T {
        self.data_ref()
    }
}

impl<T: ?Sized> Deref for Lich<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data_ref()
    }
}

impl<T: ?Sized> AsRef<T> for Lich<T> {
    fn as_ref(&self) -> &T {
        self.data_ref()
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for Lich<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lich")
            .field("value", &self.data_ref())
            .field("bindings", &self.bindings())
            .finish()
    }
}

impl<T: fmt::Display + ?Sized> fmt::Display for Lich<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.data_ref(), f)
    }
}

impl<T: ?Sized> fmt::Pointer for Lich<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.value.as_ptr(), f)
    }
}

impl<T: PartialEq + ?Sized> PartialEq for Lich<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data_ref() == other.data_ref()
    }
}

impl<T: Eq + ?Sized> Eq for Lich<T> {}

impl<T: PartialOrd + ?Sized> PartialOrd for Lich<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        PartialOrd::partial_cmp(self.data_ref(), other.data_ref())
    }
}

impl<T: Ord + ?Sized> Ord for Lich<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Ord::cmp(self.data_ref(), other.data_ref())
    }
}

impl<T: core::hash::Hash + ?Sized> core::hash::Hash for Lich<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.data_ref().hash(state);
    }
}

impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        // Safety: this `Lich` is no longer externally reachable since it is being
        // dropped.
        unsafe { self.redeem_unchecked() };
    }
}

pub(crate) fn increment(count: &AtomicU32) -> u32 {
    let result = count.fetch_update(Ordering::Acquire, Ordering::Relaxed, |value| {
        if value < SEVERED - 1 {
            Some(value + 1)
        } else {
            None
        }
    });
    match result {
        Ok(value) => value,
        // `Err(SEVERED)` means `sever` has already been called. `bind` requires a
        // `Pin<&Self>` which is impossible to hold after `sever` consumes the Pin,
        // so this branch is unreachable in safe code.
        Err(SEVERED) => unreachable!("bind called on a severed Soul"),
        Err(_) => panic!("maximum number of `Lich`es reached"),
    }
}

pub(crate) fn decrement(count: &AtomicU32) -> u32 {
    match count.fetch_sub(1, Ordering::Relaxed) {
        0 | SEVERED => unreachable!(),
        value => value - 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::hash::{Hash, Hasher};
    use core::mem::ManuallyDrop;

    /// Helper: create a `Lich<T>` that points directly at `value` with the
    /// given shared atomic count. Returned Lich is wrapped in `ManuallyDrop`
    /// to avoid touching the count on drop (since the count is stack-local).
    fn test_lich<T>(value: &T, count: &AtomicU32) -> ManuallyDrop<Lich<T>> {
        count.fetch_add(1, Ordering::Relaxed);
        ManuallyDrop::new(Lich {
            value: NonNull::from(value),
            count: NonNull::from(count),
        })
    }

    #[test]
    fn partial_eq_equal_values() {
        let count = AtomicU32::new(0);
        let a = test_lich(&42_i32, &count);
        let b = test_lich(&42_i32, &count);
        assert_eq!(*a, *b);
    }

    #[test]
    fn partial_eq_different_values() {
        let count = AtomicU32::new(0);
        let a = test_lich(&42_i32, &count);
        let b = test_lich(&99_i32, &count);
        assert_ne!(*a, *b);
    }

    #[test]
    fn ord_and_partial_ord() {
        let count = AtomicU32::new(0);
        let a: ManuallyDrop<Lich<i32>> = test_lich(&1_i32, &count);
        let b: ManuallyDrop<Lich<i32>> = test_lich(&2_i32, &count);
        assert!(*a < *b);
        assert!(*b > *a);
        assert_eq!(a.cmp(&a), core::cmp::Ordering::Equal);
        assert_eq!(a.cmp(&b), core::cmp::Ordering::Less);
    }

    #[test]
    fn hash_equal_values() {
        let count = AtomicU32::new(0);
        let a = test_lich(&42_i32, &count);
        let b = test_lich(&42_i32, &count);

        let compute_hash = |lich: &Lich<i32>| {
            let mut hasher = std::hash::DefaultHasher::new();
            lich.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(compute_hash(&a), compute_hash(&b));
    }
}
