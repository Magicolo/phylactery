/*!
 * Abstraction layer for synchronization primitives.
 *
 * Under `cfg(loom)`, uses loom's model-checked replacements so that
 * concurrency tests can explore all possible interleavings and detect
 * memory-ordering bugs.
 */

#[cfg(not(loom))]
pub(crate) use core::sync::atomic::{AtomicU32, Ordering};
#[cfg(loom)]
pub(crate) use loom::sync::atomic::{AtomicU32, Ordering};

#[cfg(not(loom))]
pub(crate) fn wait(key: &AtomicU32, value: u32) {
    atomic_wait::wait(key, value);
}

#[cfg(loom)]
pub(crate) fn wait(key: &AtomicU32, value: u32) {
    // Under loom, spin-wait with yield to let loom explore all interleavings.
    // Real futex operations are not available under loom's model checker.
    loop {
        if key.load(Ordering::Acquire) != value {
            break;
        }
        loom::thread::yield_now();
    }
}

#[cfg(not(loom))]
pub(crate) fn wake_all(key: &AtomicU32) {
    atomic_wait::wake_all(key);
}

#[cfg(loom)]
pub(crate) fn wake_all(_key: &AtomicU32) {
    // Under loom, waiters spin-yield, so an explicit wake is a no-op.
}
