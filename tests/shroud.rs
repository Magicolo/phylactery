use phylactery::{
    lock::{Lich, Soul, ritual},
    shroud,
};

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
