#![cfg(loom)]

use loom::cell::UnsafeCell;
use loom::sync::atomic::{AtomicU32, Ordering};
use loom::sync::Arc;
use loom::thread;

/// Reproduces the memory-ordering bug described in issue 02.
///
/// The `decrement` function in `lich.rs` originally used `Ordering::Relaxed`
/// for `fetch_sub`.  The `sever` function uses `Ordering::Acquire` on its
/// successful `compare_exchange`.  Under the C++ memory model a `Relaxed`
/// store does **not** synchronise-with an `Acquire` load, so writes
/// performed by a Lich holder before dropping it are not guaranteed to be
/// visible to the Soul thread after `sever` succeeds.
///
/// Loom's `UnsafeCell` tracks unsynchronised accesses and will panic if
/// a read observes a write without a proper happens-before chain.
#[test]
#[should_panic(expected = "Causality violation")]
fn relaxed_decrement_lacks_synchronization() {
    loom::model(|| {
        let data = Arc::new(UnsafeCell::new(0u32));
        // count = 1: one simulated Lich binding.
        let count = Arc::new(AtomicU32::new(1));

        let data_lich = data.clone();
        let count_lich = count.clone();

        // --- Thread A (Lich holder) ---
        let handle = thread::spawn(move || {
            // Simulate a write through interior mutability (via &T).
            unsafe { data_lich.with_mut(|p| *p = 42) };

            // Simulate `Lich::drop` → the old (buggy) decrement with Relaxed.
            count_lich.fetch_sub(1, Ordering::Relaxed);
        });

        // --- Main thread (Soul owner / sever) ---
        // Spin until count reaches 0, mirroring the `sever` CAS loop.
        loop {
            match count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => break,
                Err(_) => thread::yield_now(),
            }
        }

        // After sever succeeds, read the shared data.  This mirrors what
        // `Drop for T` would do when the Soul drops its value.
        //
        // Without a Release in the decrement and Acquire in the CAS forming
        // a synchronises-with pair, loom should flag this access.
        let val = unsafe { data.with(|p| *p) };
        assert_eq!(val, 42);

        handle.join().unwrap();
    });
}

/// The same scenario but with `Release` ordering on the decrement.
/// This should pass because Release + Acquire forms a proper
/// synchronises-with pair.
#[test]
fn release_decrement_is_synchronized() {
    loom::model(|| {
        let data = Arc::new(UnsafeCell::new(0u32));
        let count = Arc::new(AtomicU32::new(1));

        let data_lich = data.clone();
        let count_lich = count.clone();

        let handle = thread::spawn(move || {
            unsafe { data_lich.with_mut(|p| *p = 42) };
            // Fixed: Release ordering synchronises with the Acquire CAS.
            count_lich.fetch_sub(1, Ordering::Release);
        });

        loop {
            match count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => break,
                Err(_) => thread::yield_now(),
            }
        }

        let val = unsafe { data.with(|p| *p) };
        assert_eq!(val, 42);

        handle.join().unwrap();
    });
}

/// Regression test using the real Soul/Lich API under loom.
///
/// Spawns a thread that calls a Lich then drops it.  The main thread drops
/// the Soul, which internally calls sever.  Loom explores all interleavings
/// to verify the drop protocol completes without deadlock or panic.
#[test]
fn soul_lich_drop_is_synchronized() {
    use core::pin::Pin;
    use loom::sync::atomic::AtomicBool;
    use phylactery::Soul;

    loom::model(|| {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let writer = move || {
            called_clone.store(true, Ordering::Release);
        };

        let soul: Pin<std::boxed::Box<Soul<_>>> = std::boxed::Box::pin(Soul::new(writer));
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

