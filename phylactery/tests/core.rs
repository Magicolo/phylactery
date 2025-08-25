use core::ops::Deref;

#[cfg(any(feature = "lock", feature = "cell"))]
macro_rules! lock_cell {
    () => {
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
            assert_eq!(lich.try_sever().ok(), Some(false));
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
        fn can_create_default_lich() {
            let lich = Lich::<()>::default();
            assert!(!lich.try_sever().ok().unwrap());
            let lich = Lich::<()>::default();
            assert!(!lich.sever());
        }

        #[test]
        fn can_not_borrow_after_lich_sever() {
            let function = || {};
            let (lich1, soul) = ritual::<_, dyn Fn()>(&function);
            let lich2 = lich1.clone();
            assert!(lich1.sever());
            assert!(lich2.borrow().is_none());
            assert!(redeem(lich2, soul).ok().flatten().is_none());
        }

        #[test]
        fn can_not_borrow_after_soul_sever() {
            let function = || {};
            let (lich, soul) = ritual::<_, dyn Fn()>(&function);
            assert!(soul.sever());
            assert!(lich.borrow().is_none());
            assert!(!lich.sever());
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
    };
}

#[cfg(any(feature = "lock", feature = "cell", feature = "atomic"))]
macro_rules! lock_cell_atomic {
    ([$($location: ident)?]) => {
        #[test]
        fn redeem_fails_with_some() {
            let function = || {};
            $(let mut $location = 0;)?
            let (lich1, soul1) = ritual::<_, dyn Fn()>(&function $(, &mut $location)?);
            $(let mut $location = 0;)?
            let (lich2, soul2) = ritual::<_, dyn Fn()>(&function $(, &mut $location)?);
            let (lich1, soul2) = redeem(lich1, soul2).err().unwrap();
            let (lich2, soul1) = redeem(lich2, soul1).err().unwrap();
            assert!(redeem(lich1, soul1).ok().flatten().is_none());
            assert!(redeem(lich2, soul2).ok().flatten().is_none());
        }

        #[test]
        fn can_clone_lich() {
            let function = || {};
            $(let mut $location = 0;)?
            let (lich1, soul) = ritual::<_, dyn Fn()>(&function $(, &mut $location)?);
            let lich2 = lich1.clone();
            let soul = redeem(lich1, soul).ok().flatten().unwrap();
            assert!(redeem(lich2, soul).ok().flatten().is_none());
        }

        #[test]
        fn can_redeem_in_any_order() {
            let function = || {};
            $(let mut $location = 0;)?
            let (lich1, soul) = ritual::<_, dyn Fn()>(&function $(, &mut $location)?);
            let lich2 = lich1.clone();
            let lich3 = lich2.clone();
            let lich4 = lich1.clone();
            let soul = redeem(lich2, soul).ok().flatten().unwrap();
            let soul = redeem(lich3, soul).ok().flatten().unwrap();
            let soul = redeem(lich1, soul).ok().flatten().unwrap();
            assert!(redeem(lich4, soul).ok().flatten().is_none());
        }

        #[test]
        fn is_bound() {
            let function = || {};
            $(let mut $location = 0;)?
            let (lich, soul) = ritual::<_, dyn Fn()>(&function $(, &mut $location)?);
            assert!(lich.is_bound());
            assert!(soul.is_bound());
            assert!(redeem(lich, soul).ok().flatten().is_none());
        }
    };
}

macro_rules! lock_atomic_raw {
    ([$($safe: ident)?] [$($unwrap: ident)?] [$ok: expr] [$($location: ident)?]) => {
        #[test]
        fn can_send_to_thread() {
            let function = || 'a';
            $(let mut $location = 0;)?
            let (lich, soul) = ritual::<_, dyn Fn() -> char + Send + Sync>(&function $(, &mut $location)?);
            let lich = spawn(move || {
                let lich = lich;
                assert_eq!($($safe)? { lich.borrow() }$(.$unwrap())?(), 'a');
                lich
            })
            .join()
            .unwrap();
            assert!($ok(redeem(lich, soul)));
        }

        #[test]
        fn can_be_stored_as_static() {
            static LICH: Mutex<Option<Lich<dyn Fn() -> char + Send + Sync>>> = Mutex::new(None);
            let function = || 'a';
            $(let mut $location = 0;)?
            let (lich, soul) = ritual(&function $(, &mut $location)?);
            assert!(LICH.lock().unwrap().replace(lich).is_none());
            assert_eq!(
                $($safe)? { LICH.lock().unwrap().as_ref().unwrap().borrow() }$(.$unwrap())?(),
                'a'
            );
            let lich = LICH.lock().unwrap().take().unwrap();
            assert!($ok(redeem(lich, soul)));
        }
    };
}

macro_rules! lock_cell_atomic_raw {
    ([$($safe: ident)?] [$($unwrap: ident)?] [$ok: expr] [$($location: ident)?]) => {
        #[test]
        fn redeem_succeeds_with_none() {
            let function = || {};
            $(let mut $location = 0;)?
            let (lich, soul) = ritual::<_, dyn Fn()>(&function $(, &mut $location)?);
            assert!($ok(redeem(lich, soul)));
        }

        #[test]
        fn chain_liches() {
            let function = || 'a';
            $(let mut $location = 0;)?
            let (lich1, soul1) = ritual::<_, dyn Fn() -> char>(&function $(, &mut $location)?);
            {
                let guard = $($safe)? { lich1.borrow() }$(.$unwrap())?;
                $(let mut $location = 0;)?
                let (lich2, soul2) = ritual::<_, dyn Fn() -> char>(guard.deref() $(, &mut $location)?);
                assert_eq!($($safe)? { lich2.borrow() }$(.$unwrap())?(), 'a');
                assert!($ok(redeem(lich2, soul2)));
            }
            assert!($ok(redeem(lich1, soul1)));
        }
    };
}

#[cfg(feature = "lock")]
mod lock {
    use super::*;
    use phylactery::lock::{Lich, redeem, ritual};
    use std::{sync::Mutex, thread::spawn};

    lock_cell_atomic_raw!([][unwrap][|result: Result<_, _>| result.ok().flatten().is_none()][]);
    lock_atomic_raw!([][unwrap][|result: Result<_, _>| result.ok().flatten().is_none()] []);
    lock_cell_atomic!([]);
    lock_cell!();
}

#[cfg(feature = "cell")]
mod cell {
    use super::*;
    use core::cell::RefCell;
    use phylactery::cell::{Lich, redeem};

    // lock_cell_atomic_raw!([][unwrap][|result: Result<_, _>|
    // result.ok().flatten().is_none()][]); lock_cell_atomic!([]);
    // lock_cell!();

    #[test]
    fn can_be_stored_as_static() {
        thread_local! {
            static LICH: RefCell<Option<Lich<dyn Fn() -> char + Send>>> = RefCell::new(None);
        }
        let function = || 'a';
        let (lich, soul) = bind(&function);
        assert!(LICH.with_borrow_mut(|slot| slot.replace(lich)).is_none());
        assert_eq!(
            LICH.with_borrow(|slot| slot.as_ref().unwrap().get().unwrap()()),
            'a'
        );
        let lich = LICH.with_borrow_mut(|slot| slot.take()).unwrap();
        assert!(redeem(lich, soul).ok().flatten().is_none());
    }

    #[test]
    #[should_panic]
    fn panics_if_soul_is_dropped_while_borrow_lives() {
        let function = || {};
        let (lich, soul) = bind::<_, dyn Fn()>(&function);
        let guard = lich.borrow().unwrap();
        drop(soul);
        drop(guard);
    }
}

#[cfg(feature = "atomic")]
mod atomic {
    use super::*;
    use phylactery::atomic::{Lich, Soul};
    use std::{sync::Mutex, thread::spawn};

    // lock_cell_atomic_raw!([][][|result: Result<_, _>| result.is_ok()][location]);
    // lock_atomic_raw!([][][|result: Result<_, _>| result.is_ok()] [location]);
    // lock_cell_atomic!([location]);

    #[test]
    fn can_try_sever_soul() {
        let function = || {};
        let mut location = 0;
        let soul = Soul::new(&function, &mut location);
        let lich = soul.bind::<dyn Fn()>();
        let soul = soul.try_sever().err().unwrap();
        assert_eq!(soul.redeem(lich).ok(), Some(true));
        assert!(soul.try_sever().is_ok());
    }

    #[test]
    fn sever_returns_pointer() {
        let function = || 'a';
        let mut location = 0;
        let soul = Soul::new(&function, &mut location);
        let function = soul.sever();
        assert_eq!(function(), 'a');
    }
}

mod raw {
    use phylactery::raw::{Lich, Soul};
    use std::{sync::Mutex, thread::spawn};

    // lock_atomic_raw!([unsafe][][|result: Result<_, _>| result.is_ok()] []);

    macro_rules! with {
        ($module: ident, let $name: ident = $value: expr; $pointer: expr, $bind: ty) => {
            mod $module {
                use super::*;
                #[test]
                fn redeem_succeeds_with_mixed() {
                    // `Lich<T, Raw>/Soul<'a, Raw>` can not differentiate between two instances of
                    // the same binding.
                    let $name = $value;
                    let mut soul1 = Soul::new($pointer);
                    let mut soul2 = Soul::new($pointer);
                    let lich1 = soul1.bind::<$bind>();
                    let lich2 = soul2.bind::<$bind>();
                    assert!(soul2.redeem(lich1).is_ok());
                    assert!(soul1.redeem(lich2).is_ok());
                }

                #[test]
                fn does_not_panic_when_unbound() {
                    let $name = $value;
                    Soul::new($pointer);
                }

                #[test]
                #[should_panic]
                fn panics_when_lich_drops() {
                    let $name = $value;
                    let mut soul = Soul::new($pointer);
                    let lich = soul.bind::<$bind>();
                    drop(lich);
                    drop(soul);
                }

                #[test]
                #[should_panic]
                fn panics_when_soul_drops() {
                    let $name = $value;
                    let mut soul = Soul::new($pointer);
                    let lich = soul.bind::<$bind>();
                    drop(soul);
                    drop(lich);
                }

                #[test]
                fn redeem_succeeds_with_none() {
                    let $name = $value;
                    let mut soul = Soul::new($pointer);
                    let lich = soul.bind::<$bind>();
                    assert!(soul.redeem(lich).is_ok());
                }

                #[test]
                fn chain_liches() {
                    let $name = $value;
                    let mut soul1 = Soul::new($pointer);
                    let lich1 = soul1.bind::<$bind>();
                    {
                        let guard1 = unsafe { lich1.get() };
                        let mut soul2 = Soul::new(guard1);
                        let lich2 = soul2.bind::<$bind>();
                        let guard2 = unsafe { lich2.get() };
                        assert_eq!(guard1(), guard2());
                        assert!(soul2.redeem(lich2).is_ok());
                    }
                    assert!(soul1.redeem(lich1).is_ok());
                }

                #[test]
                fn can_send_to_thread() {
                    let $name = $value;
                    let value = $name();
                    let mut soul = Soul::new($pointer);
                    let lich = soul.bind::<$bind>();
                    let lich = spawn(move || {
                        let lich = lich;
                        assert_eq!(unsafe { lich.get() }(), value);
                        lich
                    })
                    .join()
                    .unwrap();
                    assert!(soul.redeem(lich).is_ok());
                }

                #[test]
                fn can_be_stored_as_static() {
                    static LICH: Mutex<Option<Lich<$bind>>> = Mutex::new(None);
                    let $name = $value;
                    let value = $name();
                    let mut soul = Soul::new($pointer);
                    let lich = soul.bind::<$bind>();
                    assert!(LICH.lock().unwrap().replace(lich).is_none());
                    assert_eq!(
                        unsafe { LICH.lock().unwrap().as_ref().unwrap().get() }(),
                        value
                    );
                    let lich = LICH.lock().unwrap().take().unwrap();
                    assert!(soul.redeem(lich).is_ok());
                }
            }
        };
    }

    with!(refed, let value = || {}; &value, dyn Fn() + Send + Sync);
    #[cfg(feature = "std")]
    with!(boxes, let value = || {}; Box::new(value), dyn Fn() + Send + Sync);
    #[cfg(feature = "std")]
    with!(rc, let value = || {}; std::rc::Rc::new(value), dyn Fn() + Send + Sync);
    #[cfg(feature = "std")]
    with!(arc, let value = || {}; std::sync::Arc::new(value), dyn Fn() + Send + Sync);
}
