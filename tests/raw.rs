use phylactery::raw::{Lich, redeem, ritual};
use std::{sync::Mutex, thread::spawn};

#[test]
fn redeem_succeeds_with_none() {
    let function = || {};
    let (lich, soul) = ritual::<_, dyn Fn()>(&function);
    assert!(unsafe { redeem(lich, soul) }.is_none());
}

#[test]
fn redeem_succeeds_with_mixed() {
    // `Raw` order `Lich<T>/Soul<'a>` can not differentiate between two instances of
    // the same binding.
    let function = || {};
    let (lich1, soul1) = ritual::<_, dyn Fn()>(&function);
    let (lich2, soul2) = ritual::<_, dyn Fn()>(&function);
    assert!(unsafe { redeem(lich1, soul2) }.is_none());
    assert!(unsafe { redeem(lich2, soul1) }.is_none());
}

#[test]
fn chain_liches() {
    let function = || 'a';
    let (lich1, soul1) = ritual::<_, dyn Fn() -> char>(&function);
    {
        let guard = unsafe { lich1.borrow() };
        let (lich2, soul2) = ritual::<_, dyn Fn() -> char>(guard);
        assert_eq!(unsafe { lich2.borrow() }(), 'a');
        assert!(unsafe { redeem(lich2, soul2) }.is_none());
    }
    assert!(unsafe { redeem(lich1, soul1) }.is_none());
}

#[test]
fn can_send_to_thread() {
    let function = || 'a';
    let (lich, soul) = ritual::<_, dyn Fn() -> char + Send + Sync>(&function);
    let lich = spawn(move || {
        let lich = lich;
        assert_eq!(unsafe { lich.borrow() }(), 'a');
        lich
    })
    .join()
    .unwrap();
    assert!(unsafe { redeem(lich, soul) }.is_none());
}

#[test]
fn can_be_stored_as_static() {
    static LICH: Mutex<Option<Lich<dyn Fn() -> char + Send + Sync>>> = Mutex::new(None);
    let function = || 'a';
    let (lich, soul) = ritual(&function);
    assert!(LICH.lock().unwrap().replace(lich).is_none());
    assert_eq!(
        unsafe { LICH.lock().unwrap().as_ref().unwrap().borrow() }(),
        'a'
    );
    let lich = LICH.lock().unwrap().take().unwrap();
    assert!(unsafe { redeem(lich, soul) }.is_none());
}
