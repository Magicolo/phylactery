#![cfg(feature = "cell")]

use core::{cell::RefCell, ops::Deref};
use phylactery::cell::{Lich, redeem, ritual};

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
fn can_be_stored_as_static() {
    thread_local! {
        static LICH: RefCell<Option<Lich<dyn Fn() -> char + Send>>> = RefCell::new(None);
    }
    let function = || 'a';
    let (lich, soul) = ritual(&function);
    assert!(LICH.with_borrow_mut(|slot| slot.replace(lich)).is_none());
    assert_eq!(
        LICH.with_borrow(|slot| slot.as_ref().unwrap().borrow().unwrap()()),
        'a'
    );
    let lich = LICH.with_borrow_mut(|slot| slot.take()).unwrap();
    assert!(redeem(lich, soul).is_none());
}
