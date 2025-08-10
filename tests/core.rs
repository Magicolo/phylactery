use core::ops::Deref;

#[cfg(any(feature = "lock", feature = "cell"))]
macro_rules! lock_cell {
    () => {
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
            assert!($($safe)? { redeem(lich, soul) }.is_none());
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
            assert!($($safe)? { redeem(lich, soul) }.is_none());
        }
    };
}

macro_rules! lock_cell_raw {
    ([$($safe: ident)?] [$($unwrap: ident)?]) => {
        #[test]
        fn redeem_succeeds_with_none() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert!($($safe)? { redeem(lich, soul) }.is_none());
        }

        #[test]
        fn chain_liches() {
            let function = || 'a';
            let (lich1, soul1) = ritual::<_, dyn Fn() -> char>(&function);
            {
                let guard = $($safe)? { lich1.borrow() }$(.$unwrap())?;
                let (lich2, soul2) = ritual::<_, dyn Fn() -> char>(guard.deref());
                assert_eq!($($safe)? { lich2.borrow() }$(.$unwrap())?(), 'a');
                assert!($($safe)? { redeem(lich2, soul2) }.is_none());
            }
            assert!($($safe)? { redeem(lich1, soul1) }.is_none());
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
        assert!(redeem(lich, soul).is_none());
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
        assert!(unsafe { redeem(lich1, soul2) }.is_none());
        assert!(unsafe { redeem(lich2, soul1) }.is_none());
    }
}
