use crate::{Binding, lich::Lich, shroud::Shroud};
use core::{
    borrow::Borrow,
    marker::PhantomPinned,
    mem::{ManuallyDrop, forget},
    ops::Deref,
    pin::Pin,
    ptr::{self, NonNull, drop_in_place, read},
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
/// implementation will either block the current thread until all [`Lich`]es are
/// dropped, or it will panic. This behavior depends on the chosen [`Binding`]
/// and guarantees that no [`Lich`] can ever outlive the data it points to.
#[derive(Debug)]
pub struct Soul<T: ?Sized, B: Binding> {
    _marker: PhantomPinned,
    bind: B,
    value: T,
}

impl<T, B: Binding> Soul<T, B> {
    pub const fn new(value: T) -> Self {
        Self {
            value,
            bind: B::NEW,
            _marker: PhantomPinned,
        }
    }

    pub fn consume(self) -> T {
        // No need to run `<Soul as Drop>::drop` since no `Lich` can be bound, given by
        // this unpinned `Soul`.
        let mut soul = ManuallyDrop::new(self);
        unsafe { drop_in_place(&mut soul.bind) };
        unsafe { read(&soul.value) }
    }
}

impl<T: ?Sized, B: Binding> Soul<T, B> {
    /// Creates a new [`Lich`] bound to this [`Soul`].
    ///
    /// This method can only be called on a pinned [`Soul`], which guarantees
    /// that the [`Soul`]'s memory location is stable.
    pub fn bind<S: Shroud<T> + ?Sized>(self: Pin<&Self>) -> Lich<S, B> {
        self.bind.increment();
        Lich {
            bind: self.bind_ptr(),
            value: S::shroud(self.value_ptr()),
        }
    }

    /// Returns `true` if the [`Lich`] has been bound by this [`Soul`]'s
    /// [`bind`](Soul::bind) method.
    pub fn is_bound<S: ?Sized>(&self, lich: &Lich<S, B>) -> bool {
        ptr::eq(&self.bind, lich.bind.as_ptr())
    }

    /// Returns the number of [`Lich`]es that are currently bound to this
    /// [`Soul`].
    pub fn bindings(&self) -> usize {
        self.bind.count() as _
    }

    /// Disposes of a [`Lich`] that was bound to this [`Soul`]. While not
    /// required, returning the [`Lich`]es explicitly to the [`Soul`] ensures
    /// that they will all be dropped when the [`Soul`] is dropped.
    ///
    /// If the [`Lich`] was not bound to this [`Soul`], it is returned as an
    /// [`Err`].
    pub fn redeem<S: ?Sized>(&self, lich: Lich<S, B>) -> Result<usize, Lich<S, B>> {
        if self.is_bound(&lich) {
            forget(lich);
            Ok(self.bind.decrement() as _)
        } else {
            Err(lich)
        }
    }

    /// Severs all bindings to [`Lich`]es from this [`Soul`], returning the
    /// unpinned [`Soul`].
    pub fn sever<S: Deref<Target = Self>>(this: Pin<S>) -> S {
        if this.bind.sever::<true>() {
            // Safety: all bindings have been severed, guaranteed by `B::sever`.
            unsafe { Self::unpin(this) }
        } else {
            panic!("sever failed possibly due to unwinding")
        }
    }

    /// Attempts to sever all bindings to [`Lich`]es from this [`Soul`],
    /// returning the unpinned [`Soul`].
    pub fn try_sever<S: Deref<Target = Self>>(this: Pin<S>) -> Result<S, Pin<S>> {
        if this.bind.sever::<false>() {
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
        // is guaranteed by `B: Binding`
        unsafe { NonNull::new_unchecked(&self.value as *const _ as _) }
    }

    fn bind_ptr(self: Pin<&Self>) -> NonNull<B> {
        // Safety: because `Soul` is pinned, it is safe to take pointers to it given
        // that those pointers are no longer accessible if the `Soul` is dropped which
        // is guaranteed by `B: Binding`
        unsafe { NonNull::new_unchecked(&self.bind as *const _ as _) }
    }
}

impl<T: ?Sized, B: Binding> Deref for Soul<T, B> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: ?Sized, B: Binding> AsRef<T> for Soul<T, B> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized, B: Binding> Borrow<T> for Soul<T, B> {
    fn borrow(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized, B: Binding> Drop for Soul<T, B> {
    fn drop(&mut self) {
        self.bind.sever::<true>();
    }
}
