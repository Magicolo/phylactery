//! A [`Binding`] variant suitable for multi-threaded scenarios.
//!
//! This variant can be sent to other threads. See the
//! [crate-level documentation](crate) for more details.

use crate::{lich, soul, Binding};
use core::sync::atomic::{AtomicU32, Ordering};

/// A [`Binding`] that uses an [`AtomicU32`] as a reference counter.
#[derive(Debug)]
#[repr(transparent)]
pub struct Atomic(AtomicU32);
pub type Lich<T> = lich::Lich<T, Atomic>;
pub type Soul<T> = soul::Soul<T, Atomic>;

unsafe impl Binding for Atomic {
    const NEW: Self = Self(AtomicU32::new(0));

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

    fn count(&self) -> u32 {
        match self.0.load(Ordering::Relaxed) {
            0 | u32::MAX => 0,
            count => count,
        }
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

    fn bail(_: *const Self) -> bool {
        false
    }
}
