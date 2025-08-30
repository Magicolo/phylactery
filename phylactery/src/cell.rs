//! A [`Binding`] variant suitable for single-threaded scenarios.
//!
//! This variant can not be sent to other threads. See the
//! [crate-level documentation](crate) for more details.

use crate::{lich, soul, Binding};

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
            value if FORCE => panic(self, value),
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

    fn bail(_this: *const Self) -> bool {
        #[cfg(feature = "std")]
        return std::thread::panicking();
        #[cfg(not(feature = "std"))]
        {
            use core::sync::atomic::Ordering;
            let address = _this.cast::<()>() as usize;
            match PANIC.load(Ordering::Relaxed) {
                0 => false,
                panic if panic == address => {
                    if COUNT.fetch_sub(1, Ordering::Relaxed) == 1 {
                        match PANIC.compare_exchange(
                            address,
                            0,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => true,
                            Err(_) => panic_multiple_unwind(),
                        }
                    } else {
                        true
                    }
                }
                _ => panic_multiple_unwind(),
            }
        }
    }
}

#[cfg(not(feature = "std"))]
static PANIC: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(not(feature = "std"))]
static COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "cell")]
fn panic<T: ?Sized>(_pointer: *const T, value: u32) -> bool {
    #[cfg(feature = "std")]
    if std::thread::panicking() {
        false
    } else {
        panic_lich_not_redeemed(value)
    }
    #[cfg(not(feature = "std"))]
    {
        use core::sync::atomic::Ordering;
        let address = _pointer.cast::<()>() as usize;
        match PANIC.compare_exchange(0, address, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => match COUNT.compare_exchange(0, value, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => panic_lich_not_redeemed(value),
                Err(_) => panic_multiple_unwind(),
            },
            Err(panic) if panic == address => false,
            Err(_) => panic_multiple_unwind(),
        }
    }
}

fn panic_lich_not_redeemed(value: u32) -> ! {
    if value <= 1 {
        panic!("'{value}' `Lich` has not been redeemed")
    } else {
        panic!("'{value}' `Lich`es have not been redeemed")
    }
}

#[cfg(not(feature = "std"))]
fn panic_multiple_unwind() -> ! {
    panic!("multiple unwinding is not supported without the `std` feature")
}
