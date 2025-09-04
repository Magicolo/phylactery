use crate::{lich::Lich, shroud::Shroud};
use core::{
    borrow::Borrow,
    marker::PhantomPinned,
    mem::{ManuallyDrop, forget},
    ops::Deref,
    pin::Pin,
    ptr::{self, NonNull, drop_in_place, read},
    sync::atomic::{AtomicU32, Ordering},
};

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
#[derive(Debug)]
pub struct Soul<T: ?Sized> {
    _marker: PhantomPinned,
    count: AtomicU32,
    value: T,
}

impl<T> Soul<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value,
            count: AtomicU32::new(0),
            _marker: PhantomPinned,
        }
    }

    /// Consumes the [`Soul`] and returns the owned value.
    pub fn consume(self) -> T {
        // No need to run `<Soul as Drop>::drop` since no `Lich` can be bound, given by
        // the fact that this `Soul` is unpinned.
        let mut soul = ManuallyDrop::new(self);
        unsafe { drop_in_place(&mut soul.count) };
        unsafe { read(&soul.value) }
    }
}

impl<T: ?Sized> Soul<T> {
    /// Binds a new [`Lich`] to this [`Soul`].
    ///
    /// This method can only be called on a pinned [`Soul`], to guarantee that
    /// the [`Soul`]'s memory location is fixed.
    pub fn bind<S: Shroud<T> + ?Sized>(self: Pin<&Self>) -> Lich<S> {
        self.count.fetch_add(1, Ordering::Relaxed);
        Lich {
            count: self.count_ptr(),
            value: S::shroud(self.value_ptr()),
        }
    }

    /// Returns `true` if the [`Lich`] is bound to this [`Soul`].
    pub fn is_bound<S: ?Sized>(&self, lich: &Lich<S>) -> bool {
        ptr::eq(&self.count, lich.count.as_ptr())
    }

    /// Returns the number of [`Lich`]es that are currently bound to this
    /// [`Soul`].
    pub fn bindings(&self) -> usize {
        self.count
            .load(Ordering::Relaxed)
            .wrapping_add(1)
            .saturating_sub(1) as _
    }

    /// Disposes of a [`Lich`] that was bound to this [`Soul`].
    ///
    /// While not required, returning the [`Lich`]es explicitly to the [`Soul`]
    /// ensures that they will all be dropped when the [`Soul`] is dropped.
    ///
    /// If the [`Lich`] was not bound to this [`Soul`], it is returned as an
    /// [`Err`].
    pub fn redeem<S: ?Sized>(&self, lich: Lich<S>) -> Result<usize, Lich<S>> {
        if self.is_bound(&lich) {
            forget(lich);
            Ok(self.count.fetch_sub(1, Ordering::Relaxed) as _)
        } else {
            Err(lich)
        }
    }

    /// Ensures that all bindings to this [`Soul`] are severed, blocking the
    /// current thread if any bound [`Lich`] remain and returning the unpinned
    /// [`Soul`] on completion.
    pub fn sever<S: Deref<Target = Self>>(this: Pin<S>) -> S {
        if sever::<true>(&this.count) {
            // Safety: all bindings have been severed, guaranteed by `B::sever`.
            unsafe { Self::unpin(this) }
        } else {
            panic!("sever failed possibly due to unwinding")
        }
    }

    /// Returns the unpinned [`Soul`] if all bindings to it are severed.
    pub fn try_sever<S: Deref<Target = Self>>(this: Pin<S>) -> Result<S, Pin<S>> {
        if sever::<false>(&this.count) {
            // Safety: all bindings have been severed, guaranteed by `B::sever`.
            Ok(unsafe { Self::unpin(this) })
        } else {
            Err(this)
        }
    }

    /// # Safety
    ///
    /// It **must** be the case the all bindings to [`Lich`]es have been severed
    /// before calling this function.
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
        match count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
            Ok(0 | u32::MAX) | Err(u32::MAX) => break true,
            Ok(value) | Err(value) if FORCE => atomic_wait::wait(count, value),
            Ok(_) | Err(_) => break false,
        }
    }
}
