use crate::{
    lich::{Lich, increment},
    shroud::Shroud,
    sync::{self, AtomicU32, Ordering},
};
use core::{
    borrow::Borrow,
    marker::PhantomPinned,
    mem::ManuallyDrop,
    ops::Deref,
    pin::Pin,
    ptr::{self, NonNull, read},
};

/// Sentinel value written to `Soul::count` by `sever` to indicate that the
/// Soul has been permanently deactivated. `u32::MAX - 1` is the maximum
/// number of live Liches; `u32::MAX` is reserved as the dead state.
pub(crate) const SEVERED: u32 = u32::MAX;

/// The owner of a value whose lifetime is dynamically extended.
///
/// A `Soul` is the anchor for a set of [`Lich`] pointers. It takes ownership of
/// a value and ensures that the value lives long enough for all associated
/// [`Lich`]es to access it, even in `'static` contexts. It acts as the root
/// of the lifetime extension mechanism.
///
/// # Usage
///
/// A [`Soul`] is created by taking ownership of a value with [`Soul::new()`].
/// Before creating any [`Lich`]es, the [`Soul`] must be pinned (see the next
/// section). Once pinned, [`Lich`]es can be created by calling
/// [`bind()`](Soul::bind). If no [`Lich`]es have been created, the [`Soul`] can
/// be unpinned and the original value retrieved with
/// [`consume()`](Soul::consume).
///
/// # Pinning
///
/// A [`Soul`] must be pinned in memory before any [`Lich`]es can be created.
/// This is because [`Lich`]es hold a raw pointer to the data inside the
/// [`Soul`], and pinning guarantees that the [`Soul`]'s memory location will
/// not change, preventing the pointers from becoming invalid. You can pin a
/// [`Soul`] to the stack with [`pin!`](core::pin::pin) or to the heap with
/// [`Box::pin`]/[`Arc::pin`](std::sync::Arc::pin)/
/// [`Rc::pin`](std::rc::Rc::pin).
///
/// # Dropping
///
/// The [`Drop`] implementation of [`Soul`] is its core safety feature. If a
/// [`Soul`] is dropped while any of its [`Lich`]es are still alive, the drop
/// implementation will block the current thread until all [`Lich`]es are
/// dropped. This behavior guarantees that no [`Lich`] can ever outlive the data
/// it points to.
#[derive(Debug, Default)]
pub struct Soul<T: ?Sized> {
    _marker: PhantomPinned,
    count: AtomicU32,
    value: T,
}

impl<T> Soul<T> {
    #[cfg(not(loom))]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            count: AtomicU32::new(0),
            _marker: PhantomPinned,
        }
    }

    #[cfg(loom)]
    pub fn new(value: T) -> Self {
        Self {
            value,
            count: AtomicU32::new(0),
            _marker: PhantomPinned,
        }
    }

    /// Consumes the [`Soul`] and returns the owned value.
    #[must_use = "discarding the value drops it silently"]
    pub fn into_value(self) -> T {
        // No need to run `<Soul as Drop>::drop` since no `Lich` can be bound, given by
        // the fact that this `Soul` is unpinned.
        unsafe { read(&ManuallyDrop::new(self).value) }
    }
}

impl<T: ?Sized> Soul<T> {
    /// Binds a new [`Lich`] to this [`Soul`].
    ///
    /// This method can only be called on a pinned [`Soul`], to guarantee that
    /// the [`Soul`]'s memory location is fixed.
    #[must_use = "the Lich is immediately dropped if not used"]
    pub fn bind<S: Shroud<T> + ?Sized>(self: Pin<&Self>) -> Lich<S> {
        increment(&self.count);
        Lich {
            count: self.count_ptr(),
            value: S::shroud(self.value_ptr()),
        }
    }

    /// Returns `true` if the [`Lich`] is bound to this [`Soul`].
    #[must_use]
    pub fn is_bound<S: ?Sized>(&self, lich: &Lich<S>) -> bool {
        ptr::eq(&self.count, lich.count.as_ptr())
    }

    /// Returns the number of [`Lich`]es that are currently bound to this
    /// [`Soul`].
    ///
    /// Returns `0` both when no Liches are bound and when the [`Soul`] has
    /// already been severed.
    #[must_use]
    pub fn bindings(&self) -> usize {
        let raw = self.count.load(Ordering::Relaxed);
        // `SEVERED` (`u32::MAX`) is the severed sentinel; treat it as 0 live bindings.
        raw.wrapping_add(1).saturating_sub(1) as _
    }

    /// Ensures that all bindings to this [`Soul`] are severed, blocking the
    /// current thread if any bound [`Lich`] remain and returning the unpinned
    /// [`Soul`] on completion.
    pub fn sever<S: Deref<Target = Self>>(this: Pin<S>) -> S {
        if sever::<true>(&this.count) {
            // Safety: `sever::<true>` returned `true`, which guarantees the atomic
            // count has been set to `u32::MAX` and all previously live Liches have
            // been dropped.  It is therefore safe to unpin the Soul.
            unsafe { Self::unpin(this) }
        } else {
            panic!("sever failed possibly due to unwinding")
        }
    }

    /// Returns the unpinned [`Soul`] if all bindings to it are severed.
    #[must_use = "if Err, the Soul has not been severed"]
    pub fn try_sever<S: Deref<Target = Self>>(this: Pin<S>) -> Result<S, Pin<S>> {
        if sever::<false>(&this.count) {
            // Safety: `sever::<false>` returned `true`, which means the CAS
            // succeeded (count was 0) and no Liches are bound.  It is therefore
            // safe to unpin the Soul.
            Ok(unsafe { Self::unpin(this) })
        } else {
            Err(this)
        }
    }

    /// # Safety
    ///
    /// The caller must ensure that `sever` (the standalone free function in
    /// this module) has returned `true` for this Soul's `count` field
    /// before calling this function.  That is, all bound [`Lich`]es must
    /// have been dropped and the `count` must have been atomically set to
    /// `u32::MAX`.
    unsafe fn unpin<S: Deref<Target = Self>>(this: Pin<S>) -> S {
        debug_assert_eq!(this.bindings(), 0);
        // Safety: no `Lich`es are bound, the `Soul` can be unpinned.
        unsafe { Pin::into_inner_unchecked(this) }
    }

    fn value_ptr(self: Pin<&Self>) -> NonNull<T> {
        // Safety: because `Soul` is pinned, it is safe to take pointers to it given
        // that those pointers are no longer accessible if the `Soul` is dropped which
        // is guaranteed by `<Soul as Drop>::drop`.
        unsafe { NonNull::new_unchecked(&self.value as *const _ as _) }
    }

    fn count_ptr(self: Pin<&Self>) -> NonNull<AtomicU32> {
        // Safety: because `Soul` is pinned, it is safe to take pointers to it given
        // that those pointers are no longer accessible if the `Soul` is dropped which
        // is guaranteed by `<Soul as Drop>::drop`.
        unsafe { NonNull::new_unchecked(&self.count as *const _ as _) }
    }
}

impl<T> From<T> for Soul<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: ?Sized> Deref for Soul<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: ?Sized> AsRef<T> for Soul<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized> Borrow<T> for Soul<T> {
    fn borrow(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized> Drop for Soul<T> {
    fn drop(&mut self) {
        sever::<true>(&self.count);
    }
}

fn sever<const FORCE: bool>(count: &AtomicU32) -> bool {
    loop {
        match count.compare_exchange(0, SEVERED, Ordering::Acquire, Ordering::Relaxed) {
            // `compare_exchange(0, …)` returns `Ok(old_value)` only when `old_value == 0`,
            // so only `Ok(0)` can appear here. `Err(SEVERED)` means a concurrent `sever`
            // already completed; either way, the Soul is severed.
            Ok(0) | Err(SEVERED) => break true,
            Ok(value) | Err(value) if FORCE => sync::wait(count, value),
            Ok(_) | Err(_) => break false,
        }
    }
}
