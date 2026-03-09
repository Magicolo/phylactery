#![cfg(loom)]

use core::pin::Pin;
use loom::{
    cell::UnsafeCell,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    thread,
};
use phylactery::Soul;

/// Regression test using the real Soul/Lich API under loom.
///
/// Spawns a thread that calls a Lich then drops it.  The main thread drops
/// the Soul, which internally calls sever.  Loom explores all interleavings
/// to verify the drop protocol completes without deadlock or panic.
#[test]
fn soul_lich_drop_is_synchronized() {
    loom::model(|| {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let writer = move || {
            called_clone.store(true, Ordering::Release);
        };

        let soul: Pin<Box<Soul<_>>> = Box::pin(Soul::new(writer));
        let lich = soul.as_ref().bind::<dyn Fn() + Send + Sync>();

        let handle = thread::spawn(move || {
            // Call the closure through the Lich.
            lich();
            // Lich drops here → decrement with Release ordering.
        });

        // Soul drops here → sever blocks (spin-yields under loom) until
        // the Lich is gone, then drops the closure.
        drop(soul);
        handle.join().unwrap();

        assert!(called.load(Ordering::Acquire));
    });
}
