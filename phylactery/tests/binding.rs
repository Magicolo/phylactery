#![cfg(feature = "shroud")]

use core::{cell::RefCell, pin::pin, time::Duration};
use phylactery::{Lich, Soul};
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    thread::{sleep, spawn},
};

#[test]
fn can_sever_unbound_soul() {
    assert_eq!(Soul::sever(Box::pin(Soul::new(|| 'a')))(), 'a');
}

#[test]
fn can_try_sever_unbound_soul() {
    assert_eq!(
        Soul::try_sever(Box::pin(Soul::new(|| 'a'))).ok().unwrap()(),
        'a'
    );
}

#[test]
fn can_not_try_sever_bound_soul() {
    let soul = Box::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn()>();
    let soul = Soul::try_sever(soul).err().unwrap();
    drop(lich);
    drop(soul);
}

#[test]
fn has_bindings() {
    let soul = Box::pin(Soul::new(|| {}));
    assert_eq!(soul.bindings(), 0);
    let lich1 = soul.as_ref().bind::<dyn Fn()>();
    assert_eq!(soul.bindings(), 1);
    assert_eq!(lich1.bindings(), 1);
    let lich2 = lich1.clone();
    assert_eq!(soul.bindings(), 2);
    assert_eq!(lich1.bindings(), 2);
    assert_eq!(lich2.bindings(), 2);
    drop(lich1);
    assert_eq!(soul.bindings(), 1);
    assert_eq!(lich2.bindings(), 1);
    assert_eq!(lich2.redeem(), 0);
    assert_eq!(soul.bindings(), 0);
}

#[test]
fn bound_lich_is_bound() {
    let soul1 = pin!(Soul::new(|| {}));
    let soul2 = pin!(Soul::new(|| {}));
    let lich = soul1.as_ref().bind::<dyn Fn()>();
    assert!(soul1.is_bound(&lich));
    assert!(!soul2.is_bound(&lich));
}

#[test]
fn can_clone_lich() {
    let soul = Box::pin(Soul::new(|| {}));
    let lich1 = soul.as_ref().bind::<dyn Fn()>();
    let lich2 = lich1.clone();
    assert_eq!(lich1.redeem(), 1);
    assert_eq!(lich2.redeem(), 0);
}

#[test]
fn can_redeem_bound_lich() {
    let soul = Box::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn()>();
    assert_eq!(lich.redeem(), 0);
}

#[test]
fn can_redeem_in_any_order() {
    let soul = Box::pin(Soul::new(|| {}));
    let lich1 = soul.as_ref().bind::<dyn Fn()>();
    let lich2 = soul.as_ref().bind::<dyn Fn()>();
    let lich3 = lich2.clone();
    assert_eq!(lich3.redeem(), 2);
    assert_eq!(lich2.redeem(), 1);
    assert_eq!(lich1.redeem(), 0);
}

#[test]
fn can_chain_liches() {
    let function = || 'a';
    let soul1 = Box::pin(Soul::new(&function));
    let lich1 = soul1.as_ref().bind::<dyn Fn() -> char>();
    let soul2 = Box::pin(Soul::new(lich1.as_ref()));
    let lich2 = soul2.as_ref().bind::<dyn Fn() -> char>();
    assert_eq!(lich1(), 'a');
    assert_eq!(lich2(), 'a');
}

#[test]
fn can_pin_on_stack() {
    let soul = pin!(Soul::new(|| 'a'));
    assert_eq!(soul(), 'a');
    let lich = soul.as_ref().bind::<dyn Fn() -> char>();
    assert_eq!(lich(), 'a');
}

#[test]
fn can_pin_with_arc() {
    let soul = Arc::pin(Soul::new(|| 'a'));
    assert_eq!(soul(), 'a');
    let lich = soul.as_ref().bind::<dyn Fn() -> char>();
    assert_eq!(lich(), 'a');
    assert_eq!(soul.bindings(), 1);
}

#[test]
fn can_pin_with_rc() {
    let soul = Rc::pin(Soul::new(|| 'a'));
    assert_eq!(soul(), 'a');
    let lich = soul.as_ref().bind::<dyn Fn() -> char>();
    assert_eq!(lich(), 'a');
    assert_eq!(soul.bindings(), 1);
}

#[test]
#[should_panic]
fn unwinds_on_same_thread() {
    let soul = Box::pin(Soul::new(|| {}));
    let _lich1 = soul.as_ref().bind::<dyn Fn() + Sync>();
    let _lich2 = _lich1.clone();
    panic!();
}

#[test]
fn can_send_to_thread() {
    let soul = Box::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn() + Sync>();
    spawn(move || {
        lich();
    });
}

#[test]
fn can_be_stored_as_static() {
    static LICH: Mutex<Option<Lich<dyn Fn() + Sync>>> = Mutex::new(None);
    let soul = Box::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn() + Sync>();
    assert!(LICH.lock().unwrap().replace(lich).is_none());
    assert!(LICH.lock().unwrap().take().is_some());
}

#[test]
fn can_be_stored_as_thread_local() {
    thread_local! {
        static LICH: RefCell<Option<Lich<dyn Fn()>>> = RefCell::new(None);
    }
    let soul = Box::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn()>();
    assert!(LICH.replace(Some(lich)).is_none());
    assert!(LICH.take().is_some());
}

#[test]
#[should_panic]
fn unwinds_on_different_threads() {
    let soul = Box::pin(Soul::new(|| {}));
    let lich1 = soul.as_ref().bind::<dyn Fn() + Sync>();
    let _lich2 = soul.as_ref().bind::<dyn Fn() + Sync>();
    spawn(move || {
        lich1();
        sleep(Duration::from_millis(100));
        let _lich3 = lich1.clone();
        panic!();
    });
    panic!();
}

// Too slow to run...
// #[test]
// #[should_panic]
// fn too_many_liches_panics() {
//     let soul = Box::pin(Soul::new(|| {}));
//     let soul = ManuallyDrop::new(soul);
//     for _ in 0..u32::MAX {
//         forget(soul.as_ref().bind::<dyn Fn()>());
//     }
// }

/// Regression test for Issue 01: `Lich::redeem` must wake a parked `sever` thread.
///
/// Before the fix, `Soul::redeem` decremented the counter without calling
/// `atomic_wait::wake_all`, so a thread blocked inside `Soul::sever` would
/// stay parked indefinitely.
#[test]
fn redeem_wakes_sever_thread() {
    use std::sync::mpsc;

    let soul: std::pin::Pin<Arc<Soul<fn()>>> = Arc::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn()>();

    // Clone the Arc so the spawned thread can call sever.
    // Pin<Arc<T>> implements Clone because Arc<T> is Clone regardless of T,
    // and Arc::clone never moves its heap-allocated T — the Pin invariant holds.
    let soul_for_sever = soul.clone();

    let (tx, rx) = mpsc::channel::<()>();

    let handle = spawn(move || {
        Soul::sever(soul_for_sever);
        let _ = tx.send(());
    });

    // Give the sever thread time to enter atomic_wait::wait.
    sleep(Duration::from_millis(30));

    // Redeem the last Lich via Lich::redeem (calls wake_all when count reaches 0).
    assert_eq!(lich.redeem(), 0, "lich should be the last binding");

    // Expect sever to complete promptly.
    rx.recv_timeout(Duration::from_millis(1000))
        .expect("sever thread should have woken up after redeem (Issue 01)");

    handle.join().unwrap();
}
