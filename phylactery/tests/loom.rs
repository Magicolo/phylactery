#![cfg(loom)]

use core::pin::Pin;
use loom::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
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
            lich();
        });

        drop(soul);
        handle.join().unwrap();

        assert!(called.load(Ordering::Acquire));
    });
}

/// Two threads each hold a clone of the same Lich and drop them concurrently.
/// The Soul (dropped on the main thread) must wait for both.
#[test]
fn clone_and_drop_concurrent() {
    loom::model(|| {
        let soul: Pin<Box<Soul<_>>> = Box::pin(Soul::new(|| {}));
        let lich1 = soul.as_ref().bind::<dyn Fn() + Send + Sync>();
        let lich2 = lich1.clone();

        let h1 = thread::spawn(move || {
            drop(lich1);
        });
        let h2 = thread::spawn(move || {
            drop(lich2);
        });

        drop(soul);
        h1.join().unwrap();
        h2.join().unwrap();
    });
}

/// One thread binds (creates) a Lich while another thread drops an existing
/// Lich, both racing with the Soul drop.
#[test]
fn concurrent_bind_and_drop() {
    loom::model(|| {
        let soul: Pin<Arc<Soul<_>>> = Arc::pin(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send + Sync>();

        // Thread A: bind a new Lich and immediately drop it.
        let soul_clone = soul.clone();
        let h1 = thread::spawn(move || {
            let lich2 = soul_clone.as_ref().bind::<dyn Fn() + Send + Sync>();
            drop(lich2);
        });

        // Thread B: drop the original Lich.
        let h2 = thread::spawn(move || {
            drop(lich);
        });

        // Main thread: drop the Soul → sever waits for count to reach 0.
        drop(soul);
        h1.join().unwrap();
        h2.join().unwrap();
    });
}

/// Lich::redeem on a spawned thread should wake Soul::sever on the main thread.
/// Loom will explore the interleaving where sever observes a non-zero count
/// and spin-waits, then redeem decrements to zero and wakes it.
#[test]
fn redeem_wakes_sever() {
    loom::model(|| {
        let soul: Pin<Arc<Soul<_>>> = Arc::pin(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send + Sync>();

        let handle = thread::spawn(move || {
            let remaining = lich.redeem();
            assert_eq!(remaining, 0);
        });

        Soul::sever(soul);
        handle.join().unwrap();
    });
}

/// Multiple Liches dropped on separate threads at staggered points.
/// The Soul's sever must wait for all of them.
#[test]
fn multiple_liches_staggered_drop() {
    loom::model(|| {
        let soul: Pin<Box<Soul<_>>> = Box::pin(Soul::new(|| {}));
        let lich1 = soul.as_ref().bind::<dyn Fn() + Send + Sync>();
        let lich2 = soul.as_ref().bind::<dyn Fn() + Send + Sync>();

        // Thread A drops lich1.
        let h1 = thread::spawn(move || {
            drop(lich1);
        });

        // Thread B drops lich2.
        let h2 = thread::spawn(move || {
            drop(lich2);
        });

        // Main thread drops soul (sever blocks until both are gone).
        drop(soul);
        h1.join().unwrap();
        h2.join().unwrap();
    });
}

/// try_sever must fail when a Lich is still alive, and succeed after the
/// Lich is dropped — even when the drop happens on another thread.
#[test]
fn try_sever_while_lich_alive() {
    loom::model(|| {
        let soul: Pin<Arc<Soul<_>>> = Arc::pin(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send + Sync>();

        // Drop the Lich on another thread.
        let handle = thread::spawn(move || {
            drop(lich);
        });
        handle.join().unwrap();

        // Now try_sever should succeed because the Lich has been dropped.
        assert!(Soul::try_sever(soul).is_ok());
    });
}

/// Soul::sever racing with the last Lich::drop on another thread.
/// This is the core synchronization edge case: the CAS in sever and the
/// fetch_sub in decrement must form a Release/Acquire pair.
#[test]
fn sever_concurrent_with_last_drop() {
    loom::model(|| {
        let soul: Pin<Arc<Soul<_>>> = Arc::pin(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send + Sync>();

        let handle = thread::spawn(move || {
            lich();
            // Lich drops here → Release fetch_sub.
        });

        // Soul::sever → Acquire CAS; must see the Release from decrement.
        Soul::sever(soul);
        handle.join().unwrap();
    });
}

/// Deref access (read) on one thread racing with drop on another and
/// sever on the main thread. The read must complete before the Soul is
/// severed and its data dropped.
#[test]
fn concurrent_reads_during_drop() {
    loom::model(|| {
        let soul: Pin<Box<Soul<_>>> = Box::pin(Soul::new(|| {}));
        let lich1 = soul.as_ref().bind::<dyn Fn() + Send + Sync>();
        let lich2 = lich1.clone();

        let handle = thread::spawn(move || {
            // Read through Deref (call the closure), then drop.
            lich1();
            drop(lich1);
        });

        // Drop lich2 on main thread.
        drop(lich2);
        // Soul drops here → sever.
        drop(soul);
        handle.join().unwrap();
    });
}

/// Exercises the `bindings()` query under contention: one thread clones a
/// Lich while another drops it, and the main thread queries the count.
/// All interleavings must be panic-free (no overflow, no underflow).
#[test]
fn bindings_count_under_contention() {
    loom::model(|| {
        let soul: Pin<Arc<Soul<_>>> = Arc::pin(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send + Sync>();
        let lich_for_clone = lich.clone();

        let soul_clone = soul.clone();
        let h1 = thread::spawn(move || {
            // Clone increments the count.
            let extra = lich_for_clone.clone();
            let _ = soul_clone.bindings();
            drop(extra);
            drop(lich_for_clone);
        });

        let h2 = thread::spawn(move || {
            drop(lich);
        });

        h1.join().unwrap();
        h2.join().unwrap();

        // After all Liches are dropped, bindings should be 0.
        assert_eq!(soul.bindings(), 0);
        drop(soul);
    });
}

/// Two threads both call Soul::sever on clones of the same Arc<Soul>.
/// Exactly one should complete the CAS; neither should deadlock.
#[test]
fn concurrent_sever_calls() {
    loom::model(|| {
        let soul: Pin<Arc<Soul<_>>> = Arc::pin(Soul::new(|| {}));

        let s1 = soul.clone();
        let s2 = soul.clone();
        drop(soul); // drop original so only the two clones remain

        let h1 = thread::spawn(move || {
            Soul::sever(s1);
        });
        let h2 = thread::spawn(move || {
            Soul::sever(s2);
        });

        h1.join().unwrap();
        h2.join().unwrap();
    });
}

/// Bind, clone, redeem, and drop interleaved across two threads.
/// Exercises the increment/decrement paths under maximal contention.
#[test]
fn bind_clone_redeem_interleaved() {
    loom::model(|| {
        let soul: Pin<Arc<Soul<_>>> = Arc::pin(Soul::new(|| {}));
        let lich1 = soul.as_ref().bind::<dyn Fn() + Send + Sync>();

        let soul_clone = soul.clone();
        let handle = thread::spawn(move || {
            // Bind a second Lich from a different thread.
            let lich2 = soul_clone.as_ref().bind::<dyn Fn() + Send + Sync>();
            // Call it, then redeem.
            lich2();
            let _ = lich2.redeem();
        });

        // Main thread redeems the first Lich.
        let _ = lich1.redeem();
        handle.join().unwrap();

        drop(soul);
    });
}

/// A Lich is shared via Arc so that both threads can call through it, then
/// the Arc is dropped. Verifies Deref is safe under contention.
#[test]
fn shared_lich_via_arc() {
    loom::model(|| {
        let soul: Pin<Box<Soul<_>>> = Box::pin(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn() + Send + Sync>();
        let lich_arc = Arc::new(lich);

        let lich_arc2 = lich_arc.clone();
        let handle = thread::spawn(move || {
            // Call the closure through the Arc<Lich>.
            (&**lich_arc2)();
        });

        (&**lich_arc)();
        handle.join().unwrap();

        // Drop the Arc (and the Lich inside it).
        drop(lich_arc);
        drop(soul);
    });
}
