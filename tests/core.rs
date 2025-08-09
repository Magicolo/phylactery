use core::{ops::Deref, time::Duration};
use phylactery::{
    lock::{Lich, Soul},
    ritual, shroud,
};
use std::{
    sync::Mutex,
    thread::{sleep, spawn},
};

fn scopeth<F: Fn() + Send + Sync>(f: &F) -> Soul<'_> {
    let (l, s): (Lich<dyn Fn() + Send + Sync + 'static>, _) = ritual(f);
    spawn(move || {
        let l = l;
        l.borrow().unwrap().deref()();
    })
    .join()
    .unwrap();
    s
}

#[test]
fn boba() {
    let a = Mutex::new('c');
    let f = || {
        *a.lock().unwrap() = 'd';
    };
    let soul = scopeth(&f);
    while soul.is_bound() {
        sleep(Duration::from_millis(1));
    }
    soul.sever();
    let a = a.into_inner().unwrap();
    assert_eq!(a, 'd');
}

#[test]
fn shroud_macro_compiles() {
    trait Simple {}
    impl Simple for () {}
    shroud!(Simple);
    trait Generic<T> {}
    impl Generic<()> for () {}
    shroud!(Generic<T>);
    trait Generics<T0, T1, T2> {}
    impl Generics<(), (), ()> for () {}
    shroud!(Generics<T0, T1, T2>);

    fn simple(value: &impl Simple) -> (Lich<dyn Simple + 'static>, Soul<'_>) {
        ritual(value)
    }
    fn generic<'a, T: 'a>(
        value: &'a impl Generic<T>,
    ) -> (Lich<dyn Generic<T> + 'static>, Soul<'a>) {
        ritual(value)
    }
    #[allow(clippy::type_complexity)]
    fn generics<'a, T0: 'a, T1: 'a, T2: 'a>(
        value: &'a impl Generics<T0, T1, T2>,
    ) -> (Lich<dyn Generics<T0, T1, T2>>, Soul<'a>) {
        ritual(value)
    }

    simple(&());
    generic(&());
    generics(&());
}
