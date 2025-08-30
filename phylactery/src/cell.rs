//! A [`Binding`] variant suitable for single-threaded scenarios.
//!
//! This variant can not be sent to other threads. See the
//! [crate-level documentation](crate) for more details.

use crate::{Binding, lich, soul};

/// A [`Binding`] that uses a [`Cell<u32>`](core::cell::Cell<u32>) as a
/// reference counter.
#[derive(Debug)]
#[repr(transparent)]
pub struct Cell(core::cell::Cell<u32>);
pub type Lich<T> = lich::Lich<T, Cell>;
pub type Soul<T> = soul::Soul<T, Cell>;

unsafe impl Binding for Cell {
    const NEW: Self = Self(core::cell::Cell::new(0));

    fn sever<const FORCE: bool>(&self) -> bool {
        match self.0.get() {
            0 => {
                self.0.set(u32::MAX);
                true
            }
            u32::MAX => true,
            value if FORCE => bind::sever(self, value),
            _ => false,
        }
    }

    fn redeem(&self) {}

    fn count(&self) -> u32 {
        match self.0.get() {
            0 | u32::MAX => 0,
            count => count,
        }
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

    fn bail(this: *const Self, drop: bool) -> bool {
        bind::bail(this, drop)
    }
}

#[cfg(feature = "std")]
mod bind {
    use super::*;
    use core::cell::RefCell;
    use std::collections::BTreeMap;

    thread_local! {
        static PANIC: RefCell<BTreeMap<usize, u32>> = const { RefCell::new(BTreeMap::new()) };
    }

    pub fn sever<T: ?Sized>(this: *const T, value: u32) -> bool {
        debug_assert!(value > 0);
        let address = this.cast::<()>() as usize;
        match PANIC.with_borrow_mut(|map| map.insert(address, value)) {
            // This `Soul` is already unwinding. This can happen with a call to `Soul::sever`.
            Some(_) => false,
            None => panic_lich_not_redeemed(value),
        }
    }

    pub fn bail<T: ?Sized>(this: *const T, drop: bool) -> bool {
        let address = this.cast::<()>() as usize;
        PANIC.with_borrow_mut(|map| match map.get_mut(&address) {
            Some(0) => unreachable!("invalid count"),
            Some(1) if drop => {
                map.remove(&address);
                true
            }
            Some(count) => {
                *count -= 1;
                true
            }
            None => false,
        })
    }
}

#[cfg(not(feature = "std"))]
mod bind {
    use super::*;
    use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering::Relaxed};

    static PANIC: AtomicUsize = AtomicUsize::new(0);
    static COUNT: AtomicU32 = AtomicU32::new(0);

    pub fn sever<T: ?Sized>(this: *const T, value: u32) -> bool {
        debug_assert!(value > 0);
        let address = this.cast::<()>() as usize;
        match PANIC.compare_exchange(0, address, Relaxed, Relaxed) {
            Ok(_) => match COUNT.compare_exchange(0, value, Relaxed, Relaxed) {
                Ok(_) => panic_lich_not_redeemed(value),
                Err(_) => unreachable!("invalid count"),
            },
            // This `Soul` is already unwinding. This can happen with a call to `Soul::sever`.
            Err(panic) if panic == address => false,
            Err(_) => panic_multiple_unwind(),
        }
    }

    pub fn bail<T: ?Sized>(this: *const T, drop: bool) -> bool {
        let address = this.cast::<()>() as usize;
        match PANIC.load(Relaxed) {
            0 => false,
            panic if drop && panic == address => match COUNT.fetch_sub(1, Relaxed) {
                0 => unreachable!("invalid count"),
                1 => match PANIC.compare_exchange(address, 0, Relaxed, Relaxed) {
                    Ok(_) => true,
                    Err(_) => panic_multiple_unwind(),
                },
                _ => true,
            },
            panic if panic == address => true,
            _ => panic_multiple_unwind(),
        }
    }

    fn panic_multiple_unwind() -> ! {
        panic!("multiple unwinding is not supported without the `std` feature")
    }
}

fn panic_lich_not_redeemed(value: u32) -> ! {
    if value <= 1 {
        panic!("'{value}' `Lich` has not been redeemed")
    } else {
        panic!("'{value}' `Lich`es have not been redeemed")
    }
}
