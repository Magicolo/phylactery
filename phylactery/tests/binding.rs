#![cfg(feature = "shroud")]

#[cfg(any(feature = "cell", feature = "atomic"))]
macro_rules! tests {
    () => {
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
            assert!(soul.redeem(lich2).is_ok());
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
            assert!(soul.redeem(lich1).is_ok());
            assert!(soul.redeem(lich2).is_ok());
        }

        #[test]
        fn can_redeem_bound_lich() {
            let soul = Box::pin(Soul::new(|| {}));
            let lich = soul.as_ref().bind::<dyn Fn()>();
            assert!(soul.redeem(lich).is_ok());
        }

        #[test]
        fn can_not_redeem_other_lich() {
            let soul1 = Box::pin(Soul::new(|| {}));
            let soul2 = Box::pin(Soul::new(|| {}));
            let lich1 = soul1.as_ref().bind::<dyn Fn()>();
            let lich2 = soul2.as_ref().bind::<dyn Fn()>();
            assert!(soul1.redeem(lich2).is_err());
            assert!(soul2.redeem(lich1).is_err());
        }

        #[test]
        fn can_redeem_in_any_order() {
            let soul = Box::pin(Soul::new(|| {}));
            let lich1 = soul.as_ref().bind::<dyn Fn()>();
            let lich2 = soul.as_ref().bind::<dyn Fn()>();
            let lich3 = lich2.clone();
            assert!(soul.redeem(lich3).is_ok());
            assert!(soul.redeem(lich2).is_ok());
            assert!(soul.redeem(lich1).is_ok());
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
    };
}

#[cfg(feature = "cell")]
mod cell {
    use core::{cell::RefCell, pin::pin};
    use phylactery::cell::{Lich, Soul};
    use std::{rc::Rc, sync::Arc, thread::spawn};

    tests!();

    #[test]
    fn can_be_stored_as_static() {
        thread_local! {
            static LICH: RefCell<Option<Lich<dyn Fn()>>> = RefCell::new(None);
        }
        let soul = Box::pin(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn()>();
        LICH.set(Some(lich));
        LICH.take().unwrap();
    }

    #[test]
    #[should_panic]
    fn panics_when_bound_soul_is_dropped() {
        let soul = Box::pin(Soul::new(|| {}));
        let lich = soul.as_ref().bind::<dyn Fn()>();
        drop(soul);
        drop(lich);
    }

    #[test]
    #[should_panic]
    fn panics_on_different_threads() {
        let soul1 = Box::pin(Soul::new(|| {}));
        let lich1 = soul1.as_ref().bind::<dyn Fn()>();
        spawn(|| {
            let soul2 = Box::pin(Soul::new(|| {}));
            let lich2 = soul2.as_ref().bind::<dyn Fn()>();
            drop(soul2);
            lich2();
        })
        .join()
        .unwrap_err();
        drop(soul1);
        lich1();
    }
}

#[cfg(feature = "atomic")]
mod atomic {
    use core::{pin::pin, time::Duration};
    use phylactery::atomic::{Lich, Soul};
    use std::{
        rc::Rc,
        sync::{Arc, Mutex},
        thread::{sleep, spawn},
    };

    tests!();

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
}
