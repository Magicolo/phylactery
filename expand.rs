#[rustc_test_marker = "tests::shroud_macro_compiles"]
#[doc(hidden)]
pub const shroud_macro_compiles: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("tests::shroud_macro_compiles"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 286usize,
        start_col: 8usize,
        end_line: 286usize,
        end_col: 29usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(shroud_macro_compiles()),
    ),
};
fn shroud_macro_compiles() {
    trait Simple {}
    impl<TConcrete: Simple> crate::Shroud<dyn Simple> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Simple) {
            self as *const Self as *const _
        }
    }
    impl<
        TConcrete: Simple + Send + Sync + Unpin,
    > crate::Shroud<dyn Simple + Send + Sync + Unpin> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Simple + Send + Sync + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<TConcrete: Simple + Sync + Unpin> crate::Shroud<dyn Simple + Sync + Unpin>
    for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Simple + Sync + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<TConcrete: Simple + Send + Unpin> crate::Shroud<dyn Simple + Send + Unpin>
    for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Simple + Send + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<TConcrete: Simple + Send + Sync> crate::Shroud<dyn Simple + Send + Sync>
    for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Simple + Send + Sync) {
            self as *const Self as *const _
        }
    }
    impl<TConcrete: Simple + Unpin> crate::Shroud<dyn Simple + Unpin> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Simple + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<TConcrete: Simple + Sync> crate::Shroud<dyn Simple + Sync> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Simple + Sync) {
            self as *const Self as *const _
        }
    }
    impl<TConcrete: Simple + Send> crate::Shroud<dyn Simple + Send> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Simple + Send) {
            self as *const Self as *const _
        }
    }
    trait Generic<T> {}
    impl<T, TConcrete: Generic<T>> crate::Shroud<dyn Generic<T>> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generic<T>) {
            self as *const Self as *const _
        }
    }
    impl<
        T,
        TConcrete: Generic<T> + Send + Sync + Unpin,
    > crate::Shroud<dyn Generic<T> + Send + Sync + Unpin> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generic<T> + Send + Sync + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<
        T,
        TConcrete: Generic<T> + Sync + Unpin,
    > crate::Shroud<dyn Generic<T> + Sync + Unpin> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generic<T> + Sync + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<
        T,
        TConcrete: Generic<T> + Send + Unpin,
    > crate::Shroud<dyn Generic<T> + Send + Unpin> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generic<T> + Send + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<
        T,
        TConcrete: Generic<T> + Send + Sync,
    > crate::Shroud<dyn Generic<T> + Send + Sync> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generic<T> + Send + Sync) {
            self as *const Self as *const _
        }
    }
    impl<T, TConcrete: Generic<T> + Unpin> crate::Shroud<dyn Generic<T> + Unpin>
    for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generic<T> + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<T, TConcrete: Generic<T> + Sync> crate::Shroud<dyn Generic<T> + Sync>
    for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generic<T> + Sync) {
            self as *const Self as *const _
        }
    }
    impl<T, TConcrete: Generic<T> + Send> crate::Shroud<dyn Generic<T> + Send>
    for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generic<T> + Send) {
            self as *const Self as *const _
        }
    }
    trait Generics<T0, T1, T2> {}
    impl<
        T0,
        T1,
        T2,
        TConcrete: Generics<T0, T1, T2>,
    > crate::Shroud<dyn Generics<T0, T1, T2>> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generics<T0, T1, T2>) {
            self as *const Self as *const _
        }
    }
    impl<
        T0,
        T1,
        T2,
        TConcrete: Generics<T0, T1, T2> + Send + Sync + Unpin,
    > crate::Shroud<dyn Generics<T0, T1, T2> + Send + Sync + Unpin> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generics<T0, T1, T2> + Send + Sync + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<
        T0,
        T1,
        T2,
        TConcrete: Generics<T0, T1, T2> + Sync + Unpin,
    > crate::Shroud<dyn Generics<T0, T1, T2> + Sync + Unpin> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generics<T0, T1, T2> + Sync + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<
        T0,
        T1,
        T2,
        TConcrete: Generics<T0, T1, T2> + Send + Unpin,
    > crate::Shroud<dyn Generics<T0, T1, T2> + Send + Unpin> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generics<T0, T1, T2> + Send + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<
        T0,
        T1,
        T2,
        TConcrete: Generics<T0, T1, T2> + Send + Sync,
    > crate::Shroud<dyn Generics<T0, T1, T2> + Send + Sync> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generics<T0, T1, T2> + Send + Sync) {
            self as *const Self as *const _
        }
    }
    impl<
        T0,
        T1,
        T2,
        TConcrete: Generics<T0, T1, T2> + Unpin,
    > crate::Shroud<dyn Generics<T0, T1, T2> + Unpin> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generics<T0, T1, T2> + Unpin) {
            self as *const Self as *const _
        }
    }
    impl<
        T0,
        T1,
        T2,
        TConcrete: Generics<T0, T1, T2> + Sync,
    > crate::Shroud<dyn Generics<T0, T1, T2> + Sync> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generics<T0, T1, T2> + Sync) {
            self as *const Self as *const _
        }
    }
    impl<
        T0,
        T1,
        T2,
        TConcrete: Generics<T0, T1, T2> + Send,
    > crate::Shroud<dyn Generics<T0, T1, T2> + Send> for TConcrete {
        #[inline(always)]
        fn shroud(&self) -> *const (dyn Generics<T0, T1, T2> + Send) {
            self as *const Self as *const _
        }
    }
    fn simple(value: impl Simple) {
        ritual(&value);
    }
    fn generic<T>(value: impl Generic<T>) {
        ritual(&value);
    }
    fn generics<'a, T0: 'a, T1: 'a, T2: 'a>(
        value: &'a impl Generics<T0, T1, T2>,
    ) -> (Lich<Ref<dyn Generics<T0, T1, T2> + 'static>>, Soul<'a>) {
        ritual(value)
    }
}
