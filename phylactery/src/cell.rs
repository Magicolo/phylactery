//! A [`Binding`] variant suitable for single-threaded scenarios.
//!
//! This variant can not be sent to other threads. See the
//! [crate-level documentation](crate) for more details.

use crate::{Binding, lich, panic, soul};

/// A [`Binding`] that uses a [`Cell<u32>`](core::cell::Cell<u32>) as a
/// reference counter.
#[derive(Debug)]
#[repr(transparent)]
pub struct Cell(core::cell::Cell<u32>);
pub type Lich<T> = lich::Lich<T, Cell>;
pub type Soul<P> = soul::Soul<P, Cell>;

unsafe impl Binding for Cell {
    const NEW: Self = Self(core::cell::Cell::new(0));

    fn sever<const FORCE: bool>(&self) -> bool {
        match self.0.get() {
            0 => {
                self.0.set(u32::MAX);
                true
            }
            u32::MAX => true,
            value if FORCE => panic(value),
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
}
