#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

mod lich;
pub mod shroud;
mod soul;

use crate::shroud::Shroud;
use core::ptr::NonNull;

pub unsafe trait Bind: Shroud<u32> {
    fn sever<const FORCE: bool>(&self) -> bool;
    fn redeem(&self);
    fn bindings(&self) -> u32;
    fn increment(&self) -> u32;
    fn decrement(&self) -> u32;
}

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

#[cfg(feature = "cell")]
pub mod cell {
    use super::*;

    #[repr(transparent)]
    pub struct Cell(core::cell::Cell<u32>);
    pub type Lich<T> = lich::Lich<T, Cell>;
    pub type Soul<'a, P> = soul::Soul<'a, P, Cell>;

    impl Shroud<u32> for Cell {
        fn shroud(from: NonNull<u32>) -> NonNull<Self> {
            from.cast()
        }
    }

    unsafe impl Bind for Cell {
        fn sever<const FORCE: bool>(&self) -> bool {
            match self.0.get() {
                0 | u32::MAX => {
                    self.0.set(u32::MAX);
                    true
                }
                value if FORCE => panic!("{value} `Lich<T>`es have not been redeemed"),
                _ => false,
            }
        }

        fn redeem(&self) {}

        fn bindings(&self) -> u32 {
            let count = self.0.get();
            if count == u32::MAX { 0 } else { count }
        }

        fn increment(&self) -> u32 {
            let value = self.0.get();
            assert!(value < u32::MAX - 1);
            self.0.set(value + 1);
            value
        }

        fn decrement(&self) -> u32 {
            let value = self.0.get();
            debug_assert!(value > 0);
            self.0.set(value - 1);
            value
        }
    }
}

#[cfg(feature = "lock")]
pub mod lock {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering};

    #[repr(transparent)]
    pub struct Lock(AtomicU32);
    pub type Lich<T> = lich::Lich<T, Lock>;
    pub type Soul<'a, P> = soul::Soul<'a, P, Lock>;

    impl Shroud<u32> for Lock {
        fn shroud(from: NonNull<u32>) -> NonNull<Self> {
            from.cast()
        }
    }

    #[cfg(feature = "lock")]
    unsafe impl Bind for Lock {
        fn sever<const FORCE: bool>(&self) -> bool {
            loop {
                match self
                    .0
                    .compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed)
                {
                    Ok(0 | u32::MAX) | Err(u32::MAX) => break true,
                    Ok(value) | Err(value) if FORCE => atomic_wait::wait(&self.0, value),
                    Ok(_) | Err(_) => break false,
                }
            }
        }

        fn redeem(&self) {
            atomic_wait::wake_one(&self.0);
        }

        fn bindings(&self) -> u32 {
            let count = self.0.load(Ordering::Relaxed);
            if count == u32::MAX { 0 } else { count }
        }

        fn increment(&self) -> u32 {
            let value = self.0.fetch_add(1, Ordering::Relaxed);
            assert!(value < u32::MAX - 1);
            value
        }

        fn decrement(&self) -> u32 {
            let value = self.0.fetch_sub(1, Ordering::Relaxed);
            debug_assert!(value > 0);
            value
        }
    }
}

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
