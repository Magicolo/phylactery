// use core::ops::Deref;

macro_rules! tests {
    () => {
        #[test]
        fn can_sever_unbound_soul() {
            let soul = Box::pin(Soul::new(|| 'a')).sever();
            assert_eq!(soul(), 'a');
        }

        #[test]
        fn can_try_sever_unbound_soul() {
            assert!(Box::pin(Soul::new(|| 'a')).try_sever().is_ok());
        }

        #[test]
        fn can_not_try_sever_bound_soul() {
            let soul = Box::pin(Soul::new(|| {}));
            let lich = soul.as_ref().bind::<dyn Fn()>();
            let soul = soul.try_sever().err().unwrap();
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
    };
}

#[cfg(feature = "cell")]
mod cell {
    use core::cell::RefCell;
    use phylactery::cell::{Lich, Soul};

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
}

#[cfg(feature = "lock")]
mod lock {
    use phylactery::lock::{Lich, Soul};
    use std::{sync::Mutex, thread::spawn};

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
}

// macro_rules! lock_atomic_raw {
//     ([$($safe: ident)?] [$($unwrap: ident)?] [$ok: expr] [$($location:
// ident)?]) => {         #[test]
//         fn can_send_to_thread() {
//             let function = || 'a';
//             $(let mut $location = 0;)?
//             let (lich, soul) = ritual::<_, dyn Fn() -> char + Send +
// Sync>(&function $(, &mut $location)?);             let lich = spawn(move || {
//                 let lich = lich;
//                 assert_eq!($($safe)? { lich.borrow() }$(.$unwrap())?(), 'a');
//                 lich
//             })
//             .join()
//             .unwrap();
//             assert!($ok(redeem(lich, soul)));
//         }

//         #[test]
//         fn can_be_stored_as_static() {
//             static LICH: Mutex<Option<Lich<dyn Fn() -> char + Send + Sync>>>
// = Mutex::new(None);             let function = || 'a';
//             $(let mut $location = 0;)?
//             let (lich, soul) = ritual(&function $(, &mut $location)?);
//             assert!(LICH.lock().unwrap().replace(lich).is_none());
//             assert_eq!(
//                 $($safe)? { LICH.lock().unwrap().as_ref().unwrap().borrow()
// }$(.$unwrap())?(),                 'a'
//             );
//             let lich = LICH.lock().unwrap().take().unwrap();
//             assert!($ok(redeem(lich, soul)));
//         }
//     };
// }

// macro_rules! lock_cell_atomic_raw {
//     ([$($safe: ident)?] [$($unwrap: ident)?] [$ok: expr] [$($location:
// ident)?]) => {         #[test]
//         fn redeem_succeeds_with_none() {
//             let function = || {};
//             $(let mut $location = 0;)?
//             let (lich, soul) = ritual::<_, dyn Fn()>(&function $(, &mut
// $location)?);             assert!($ok(redeem(lich, soul)));
//         }

//         #[test]
//         fn chain_liches() {
//             let function = || 'a';
//             $(let mut $location = 0;)?
//             let (lich1, soul1) = ritual::<_, dyn Fn() -> char>(&function $(,
// &mut $location)?);             {
//                 let guard = $($safe)? { lich1.borrow() }$(.$unwrap())?;
//                 $(let mut $location = 0;)?
//                 let (lich2, soul2) = ritual::<_, dyn Fn() ->
// char>(guard.deref() $(, &mut $location)?);
// assert_eq!($($safe)? { lich2.borrow() }$(.$unwrap())?(), 'a');
// assert!($ok(redeem(lich2, soul2)));             }
//             assert!($ok(redeem(lich1, soul1)));
//         }
//     };
// }

// #[cfg(feature = "lock")]
// mod lock {
//     use super::*;
//     use phylactery::lock::{Lich, redeem, ritual};
//     use std::{sync::Mutex, thread::spawn};

//     lock_cell_atomic_raw!([][unwrap][|result: Result<_, _>|
// result.ok().flatten().is_none()][]);     lock_atomic_raw!([][unwrap][|result:
// Result<_, _>| result.ok().flatten().is_none()] []);     lock_cell_atomic!
// ([]);     lock_cell!();
// }

// #[cfg(feature = "cell")]
// mod cell {
//     use super::*;
//     use core::cell::RefCell;
//     use phylactery::cell::{Lich, redeem};

//     // lock_cell_atomic_raw!([][unwrap][|result: Result<_, _>|
//     // result.ok().flatten().is_none()][]); lock_cell_atomic!([]);
//     // lock_cell!();

//     #[test]
//     fn can_be_stored_as_static() {
//         thread_local! {
//             static LICH: RefCell<Option<Lich<dyn Fn() -> char + Send>>> =
// RefCell::new(None);         }
//         let function = || 'a';
//         let (lich, soul) = bind(&function);
//         assert!(LICH.with_borrow_mut(|slot| slot.replace(lich)).is_none());
//         assert_eq!(
//             LICH.with_borrow(|slot| slot.as_ref().unwrap().get().unwrap()()),
//             'a'
//         );
//         let lich = LICH.with_borrow_mut(|slot| slot.take()).unwrap();
//         assert!(redeem(lich, soul).ok().flatten().is_none());
//     }

//     #[test]
//     #[should_panic]
//     fn panics_if_soul_is_dropped_while_borrow_lives() {
//         let function = || {};
//         let (lich, soul) = bind::<_, dyn Fn()>(&function);
//         let guard = lich.borrow().unwrap();
//         drop(soul);
//         drop(guard);
//     }
// }

// #[cfg(feature = "atomic")]
// mod atomic {
//     use super::*;
//     use phylactery::atomic::{Lich, Soul};
//     use std::{sync::Mutex, thread::spawn};

//     // lock_cell_atomic_raw!([][][|result: Result<_, _>|
// result.is_ok()][location]);     // lock_atomic_raw!([][][|result: Result<_,
// _>| result.is_ok()] [location]);     // lock_cell_atomic!([location]);

//     #[test]
//     fn can_try_sever_soul() {
//         let function = || {};
//         let mut location = 0;
//         let soul = Soul::new(&function, &mut location);
//         let lich = soul.bind::<dyn Fn()>();
//         let soul = soul.try_sever().err().unwrap();
//         assert_eq!(soul.redeem(lich).ok(), Some(true));
//         assert!(soul.try_sever().is_ok());
//     }

//     #[test]
//     fn sever_returns_pointer() {
//         let function = || 'a';
//         let mut location = 0;
//         let soul = Soul::new(&function, &mut location);
//         let function = soul.sever();
//         assert_eq!(function(), 'a');
//     }
// }

// mod raw {
//     use phylactery::raw::{Lich, Soul};
//     use std::{sync::Mutex, thread::spawn};

//     // lock_atomic_raw!([unsafe][][|result: Result<_, _>| result.is_ok()]
// []);

//     macro_rules! with {
//         ($module: ident, let $name: ident = $value: expr; $pointer: expr,
// $bind: ty) => {             mod $module {
//                 use super::*;
//                 #[test]
//                 fn redeem_succeeds_with_mixed() {
//                     // `Lich<T, Raw>/Soul<'a, Raw>` can not differentiate
// between two instances of                     // the same binding.
//                     let $name = $value;
//                     let mut soul1 = Soul::new($pointer);
//                     let mut soul2 = Soul::new($pointer);
//                     let lich1 = soul1.bind::<$bind>();
//                     let lich2 = soul2.bind::<$bind>();
//                     assert!(soul2.redeem(lich1).is_ok());
//                     assert!(soul1.redeem(lich2).is_ok());
//                 }

//                 #[test]
//                 fn does_not_panic_when_unbound() {
//                     let $name = $value;
//                     Soul::new($pointer);
//                 }

//                 #[test]
//                 #[should_panic]
//                 fn panics_when_lich_drops() {
//                     let $name = $value;
//                     let mut soul = Soul::new($pointer);
//                     let lich = soul.bind::<$bind>();
//                     drop(lich);
//                     drop(soul);
//                 }

//                 #[test]
//                 #[should_panic]
//                 fn panics_when_soul_drops() {
//                     let $name = $value;
//                     let mut soul = Soul::new($pointer);
//                     let lich = soul.bind::<$bind>();
//                     drop(soul);
//                     drop(lich);
//                 }

//                 #[test]
//                 fn redeem_succeeds_with_none() {
//                     let $name = $value;
//                     let mut soul = Soul::new($pointer);
//                     let lich = soul.bind::<$bind>();
//                     assert!(soul.redeem(lich).is_ok());
//                 }

//                 #[test]
//                 fn chain_liches() {
//                     let $name = $value;
//                     let mut soul1 = Soul::new($pointer);
//                     let lich1 = soul1.bind::<$bind>();
//                     {
//                         let guard1 = unsafe { lich1.get() };
//                         let mut soul2 = Soul::new(guard1);
//                         let lich2 = soul2.bind::<$bind>();
//                         let guard2 = unsafe { lich2.get() };
//                         assert_eq!(guard1(), guard2());
//                         assert!(soul2.redeem(lich2).is_ok());
//                     }
//                     assert!(soul1.redeem(lich1).is_ok());
//                 }

//                 #[test]
//                 fn can_send_to_thread() {
//                     let $name = $value;
//                     let value = $name();
//                     let mut soul = Soul::new($pointer);
//                     let lich = soul.bind::<$bind>();
//                     let lich = spawn(move || {
//                         let lich = lich;
//                         assert_eq!(unsafe { lich.get() }(), value);
//                         lich
//                     })
//                     .join()
//                     .unwrap();
//                     assert!(soul.redeem(lich).is_ok());
//                 }

//                 #[test]
//                 fn can_be_stored_as_static() {
//                     static LICH: Mutex<Option<Lich<$bind>>> =
// Mutex::new(None);                     let $name = $value;
//                     let value = $name();
//                     let mut soul = Soul::new($pointer);
//                     let lich = soul.bind::<$bind>();
//                     assert!(LICH.lock().unwrap().replace(lich).is_none());
//                     assert_eq!(
//                         unsafe { LICH.lock().unwrap().as_ref().unwrap().get()
// }(),                         value
//                     );
//                     let lich = LICH.lock().unwrap().take().unwrap();
//                     assert!(soul.redeem(lich).is_ok());
//                 }
//             }
//         };
//     }

//     with!(refed, let value = || {}; &value, dyn Fn() + Send + Sync);
//     #[cfg(feature = "std")]
//     with!(boxes, let value = || {}; Box::new(value), dyn Fn() + Send + Sync);
//     #[cfg(feature = "std")]
//     with!(rc, let value = || {}; std::rc::Rc::new(value), dyn Fn() + Send +
// Sync);     #[cfg(feature = "std")]
//     with!(arc, let value = || {}; std::sync::Arc::new(value), dyn Fn() + Send
// + Sync); }
