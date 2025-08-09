use core::ops::Deref;
use phylactery::lock::{Lich, redeem, ritual};
use std::{sync::Mutex, thread::spawn};

#[test]
fn redeem_succeeds_with_none() {
    let function = || {};
    let (lich, soul) = ritual::<_, dyn Fn()>(&function);
    assert!(redeem(lich, soul).is_none());
}

#[test]
fn redeem_fails_with_some() {
    let function = || {};
    let (lich1, soul1) = ritual::<_, dyn Fn()>(&function);
    let (lich2, soul2) = ritual::<_, dyn Fn()>(&function);
    let (lich1, soul2) = redeem(lich1, soul2).unwrap();
    let (lich2, soul1) = redeem(lich2, soul1).unwrap();
    assert!(redeem(lich1, soul1).is_none());
    assert!(redeem(lich2, soul2).is_none());
}

#[test]
fn chain_liches() {
    let function = || 'a';
    let (lich1, soul1) = ritual::<_, dyn Fn() -> char>(&function);
    {
        let guard = lich1.borrow().unwrap();
        let (lich2, soul2) = ritual::<_, dyn Fn() -> char>(guard.deref());
        assert_eq!(lich2.borrow().unwrap()(), 'a');
        assert!(redeem(lich2, soul2).is_none());
    }
    assert!(redeem(lich1, soul1).is_none());
}

#[test]
fn can_send_to_thread() {
    let function = || 'a';
    let (lich, soul) = ritual::<_, dyn Fn() -> char + Send>(&function);
    let lich = spawn(move || {
        let lich = lich;
        assert_eq!(lich.borrow().unwrap()(), 'a');
        lich
    })
    .join()
    .unwrap();
    assert!(redeem(lich, soul).is_none());
}

#[test]
fn can_be_stored_as_static() {
    static LICH: Mutex<Option<Lich<dyn Fn() -> char + Send>>> = Mutex::new(None);
    let function = || 'a';
    let (lich, soul) = ritual(&function);
    assert!(LICH.lock().unwrap().replace(lich).is_none());
    assert_eq!(
        LICH.lock().unwrap().as_ref().unwrap().borrow().unwrap()(),
        'a'
    );
    let lich = LICH.lock().unwrap().take().unwrap();
    assert!(redeem(lich, soul).is_none());
}
