#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "cell")]
pub mod cell;
#[cfg(feature = "lock")]
pub mod lock;
pub mod shroud;

use core::ptr::NonNull;

/// # Safety
///
/// It **must** be the case that the pointer returned by the [`AsPtr::as_ptr`]
/// method is valid, non-null, aligned and lives for as long as `Self` lives.
pub unsafe trait Pointer {
    type Target: ?Sized;
    fn pointer(&self) -> NonNull<Self::Target>;
}

/// # Safety
///
/// The implementer must guarantee that it is the only pointer to its pointee's
/// memory location.
pub unsafe trait UniquePointer: Pointer {}

/// # Safety
///
/// The implementer must guarantee that it is safe to alias its pointee's memory
/// location.
pub unsafe trait SharedPointer: Pointer + Clone {}

unsafe impl<T: ?Sized> SharedPointer for &T {}
unsafe impl<T: ?Sized> Pointer for &T {
    type Target = T;

    fn pointer(&self) -> NonNull<Self::Target> {
        unsafe { NonNull::new_unchecked(*self as *const _ as _) }
    }
}

unsafe impl<T: ?Sized> UniquePointer for &mut T {}
unsafe impl<T: ?Sized> Pointer for &mut T {
    type Target = T;

    fn pointer(&self) -> NonNull<Self::Target> {
        unsafe { NonNull::new_unchecked(*self as *const _ as _) }
    }
}

unsafe impl<T: ?Sized> SharedPointer for *const T {}
unsafe impl<T: ?Sized> Pointer for *const T {
    type Target = T;

    fn pointer(&self) -> NonNull<Self::Target> {
        NonNull::new(*self as _).expect("non-null pointer")
    }
}

unsafe impl<T: ?Sized> UniquePointer for *mut T {}
unsafe impl<T: ?Sized> Pointer for *mut T {
    type Target = T;

    fn pointer(&self) -> NonNull<Self::Target> {
        NonNull::new(*self).expect("non-null pointer")
    }
}

unsafe impl<T: ?Sized> Pointer for NonNull<T> {
    type Target = T;

    fn pointer(&self) -> NonNull<Self::Target> {
        *self
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::{rc::Rc, sync::Arc};

    unsafe impl<T: ?Sized> UniquePointer for Box<T> {}
    unsafe impl<T: ?Sized> Pointer for Box<T> {
        type Target = T;

        fn pointer(&self) -> NonNull<Self::Target> {
            unsafe { NonNull::new_unchecked(Self::as_ref(self) as *const _ as _) }
        }
    }

    unsafe impl<T: ?Sized> SharedPointer for Arc<T> {}
    unsafe impl<T: ?Sized> Pointer for Arc<T> {
        type Target = T;

        fn pointer(&self) -> NonNull<Self::Target> {
            unsafe { NonNull::new_unchecked(Self::as_ptr(self) as _) }
        }
    }

    unsafe impl<T: ?Sized> SharedPointer for Rc<T> {}
    unsafe impl<T: ?Sized> Pointer for Rc<T> {
        type Target = T;

        fn pointer(&self) -> NonNull<Self::Target> {
            unsafe { NonNull::new_unchecked(Self::as_ptr(self) as _) }
        }
    }
};

#[allow(dead_code)]
mod tests {
    macro_rules! fail {
        ($function: ident, $block: block) => {
            #[doc = concat!("```compile_fail\n", stringify!($block), "\n```")]
            const fn $function() {}
        };
    }

    fail!(can_not_drop_while_soul_lives, {
        use core::cell::RefCell;
        use phylactery::raw::ritual;

        let value = String::new();
        let cell = RefCell::new(value);
        let function = move |letter| cell.borrow_mut().push(letter);
        let (lich, soul) = ritual::<_, dyn Fn(char)>(&function);
        drop(function);
    });

    fail!(can_not_clone_lich, {
        use phylactery::raw::ritual;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn()>(&function);
        lich.clone();
    });

    fail!(can_not_clone_soul, {
        use phylactery::raw::ritual;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn()>(&function);
        soul.clone();
    });

    fail!(can_not_send_raw_unsync_to_thread, {
        use phylactery::raw::ritual;
        use std::thread::spawn;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn() + Send>(&function);
        spawn(move || lich);
    });

    fail!(can_not_create_default_raw_lich, {
        use phylactery::raw::Lich;
        Lich::<dyn Fn()>::default();
    });

    fail!(can_not_send_cell_to_thread, {
        use phylactery::cell::ritual;
        use std::thread::spawn;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn() + Send + Sync>(&function);
        spawn(move || lich);
    });

    fail!(can_not_send_lock_unsync_to_thread, {
        use phylactery::lock::ritual;
        use std::thread::spawn;

        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn() + Send>(&function);
        spawn(move || lich);
    });

    fail!(can_not_create_default_atomic_lich, {
        use phylactery::atomic::Lich;
        Lich::<dyn Fn()>::default();
    });
}
