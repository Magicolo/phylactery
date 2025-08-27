use crate::{Binding, lich, soul};
use core::sync::atomic::{AtomicU32, Ordering};

#[repr(transparent)]
pub struct Lock(AtomicU32);
pub type Lich<T> = lich::Lich<T, Lock>;
pub type Soul<P> = soul::Soul<P, Lock>;

unsafe impl Binding for Lock {
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
}
