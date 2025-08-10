use core::ops::Deref;

#[cfg(any(feature = "lock", feature = "cell"))]
macro_rules! lock_cell {
    () => {
        #[test]
        fn redeem_fails_with_some() {
            let function = || {};
            let (lich1, soul1) = ritual::<_, dyn Fn()>(&function);
            let (lich2, soul2) = ritual::<_, dyn Fn()>(&function);
            let (lich1, soul2) = redeem(lich1, soul2).err().unwrap().into_inner();
            let (lich2, soul1) = redeem(lich2, soul1).err().unwrap().into_inner();
            assert!(redeem(lich1, soul1).ok().flatten().is_none());
            assert!(redeem(lich2, soul2).ok().flatten().is_none());
        }

        #[test]
        fn can_sever_lich() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert!(lich.sever());
            assert!(!soul.sever());
        }

        #[test]
        fn can_sever_soul() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert!(soul.sever());
            assert!(!lich.sever());
        }

        #[test]
        fn can_try_sever_lich() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert_eq!(lich.try_sever().ok(), Some(true));
            assert_eq!(soul.try_sever().ok(), Some(false));
        }

        #[test]
        fn can_try_sever_soul() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert_eq!(soul.try_sever().ok(), Some(true));
            assert_eq!(lich.try_sever().ok(), Some(false));
        }

        #[test]
        fn is_not_bound_after_lich_sever() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert!(lich.sever());
            assert!(!soul.is_bound());
        }

        #[test]
        fn is_not_bound_after_soul_sever() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert!(soul.sever());
            assert!(!lich.is_bound());
        }

        #[test]
        fn can_clone_lich() {
            let function = || {};
            let (lich1, soul) = ritual::<_, dyn Fn()>(&function);
            let lich2 = lich1.clone();
            let soul = redeem(lich1, soul).ok().flatten().unwrap();
            assert!(redeem(lich2, soul).ok().flatten().is_none());
        }
    };
}

macro_rules! lock_raw {
    ([$($safe: ident)?] [$($unwrap: ident)?]) => {
        #[test]
        fn can_send_to_thread() {
            let function = || 'a';
            let (lich, soul) = ritual::<_, dyn Fn() -> char + Send + Sync>(&function);
            let lich = spawn(move || {
                let lich = lich;
                assert_eq!($($safe)? { lich.borrow() }$(.$unwrap())?(), 'a');
                lich
            })
            .join()
            .unwrap();
            assert!($($safe)? { redeem(lich, soul) }.ok().flatten().is_none());
        }

        #[test]
        fn can_be_stored_as_static() {
            static LICH: Mutex<Option<Lich<dyn Fn() -> char + Send + Sync>>> = Mutex::new(None);
            let function = || 'a';
            let (lich, soul) = ritual(&function);
            assert!(LICH.lock().unwrap().replace(lich).is_none());
            assert_eq!(
                $($safe)? { LICH.lock().unwrap().as_ref().unwrap().borrow() }$(.$unwrap())?(),
                'a'
            );
            let lich = LICH.lock().unwrap().take().unwrap();
            assert!($($safe)? { redeem(lich, soul) }.ok().flatten().is_none());
        }
    };
}

macro_rules! lock_cell_raw {
    ([$($safe: ident)?] [$($unwrap: ident)?]) => {
        #[test]
        fn redeem_succeeds_with_none() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert!($($safe)? { redeem(lich, soul) }.ok().flatten().is_none());
        }

        #[test]
        fn chain_liches() {
            let function = || 'a';
            let (lich1, soul1) = ritual::<_, dyn Fn() -> char>(&function);
            {
                let guard = $($safe)? { lich1.borrow() }$(.$unwrap())?;
                let (lich2, soul2) = ritual::<_, dyn Fn() -> char>(guard.deref());
                assert_eq!($($safe)? { lich2.borrow() }$(.$unwrap())?(), 'a');
                assert!($($safe)? { redeem(lich2, soul2) }.ok().flatten().is_none());
            }
            assert!($($safe)? { redeem(lich1, soul1) }.ok().flatten().is_none());
        }

        #[test]
        fn is_bound() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert!(lich.is_bound());
            assert!(soul.is_bound());
            assert!($($safe)? { redeem(lich, soul) }.ok().flatten().is_none());
        }
    };
}

#[cfg(feature = "lock")]
mod lock {
    use super::*;
    use phylactery::lock::{Lich, redeem, ritual};
    use std::{sync::Mutex, thread::spawn};

    lock_cell_raw!([][unwrap]);
    lock_cell!();
    lock_raw!([][unwrap]);
}

#[cfg(feature = "cell")]
mod cell {
    use super::*;
    use core::cell::RefCell;
    use phylactery::cell::{Lich, redeem, ritual};

    lock_cell_raw!([][unwrap]);
    lock_cell!();

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
        assert!(redeem(lich, soul).ok().flatten().is_none());
    }
}

mod raw {
    use super::*;
    use phylactery::raw::{Lich, redeem, ritual};
    use std::{sync::Mutex, thread::spawn};

    lock_cell_raw!([unsafe] []);
    lock_raw!([unsafe][]);

    #[test]
    fn redeem_succeeds_with_mixed() {
        // `Raw` order `Lich<T>/Soul<'a>` can not differentiate between two instances of
        // the same binding.
        let function = || {};
        let (lich1, soul1) = ritual::<_, dyn Fn()>(&function);
        let (lich2, soul2) = ritual::<_, dyn Fn()>(&function);
        assert!(unsafe { redeem(lich1, soul2) }.ok().flatten().is_none());
        assert!(unsafe { redeem(lich2, soul1) }.ok().flatten().is_none());
    }

    #[test]
    #[should_panic]
    fn panics_when_lich_is_dropped() {
        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn()>(&function);
        drop(lich);
        drop(soul);
    }

    #[test]
    #[should_panic]
    fn panics_when_soul_is_dropped() {
        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn()>(&function);
        drop(soul);
        drop(lich);
    }

    #[test]
    #[should_panic]
    fn panics_when_lich_is_severed() {
        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn()>(&function);
        lich.sever();
        drop(soul);
    }

    #[test]
    #[should_panic]
    fn panics_when_soul_is_severed() {
        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn()>(&function);
        soul.sever();
        drop(lich);
    }

    #[test]
    fn try_sever_always_fails() {
        let function = || {};
        let (lich, soul) = ritual::<_, dyn Fn()>(&function);
        let lich = lich.try_sever().unwrap_err();
        let lich = lich.try_sever().unwrap_err();
        let soul = soul.try_sever().unwrap_err();
        let soul = soul.try_sever().unwrap_err();
        assert!(unsafe { redeem(lich, soul) }.ok().flatten().is_none());
    }
}
