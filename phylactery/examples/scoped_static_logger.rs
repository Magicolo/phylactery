/// Implements a thread local scoped logger available from anywhere that can
/// borrow values that live on the stack.
#[cfg(all(feature = "cell", feature = "shroud"))]
pub mod scoped_static_logger {
    use core::{cell::RefCell, fmt::Display, pin::pin};
    use phylactery::{
        cell::{Lich, Soul},
        shroud::shroud,
    };

    // Use the convenience macro to automatically implement the required `Shroud`
    // trait for all `T: Log`.
    #[shroud]
    pub trait Log {
        fn parent(&self) -> Option<&dyn Log>;
        fn prefix(&self) -> &str;
        fn format(&self) -> &str;
        fn arguments(&self) -> &[&dyn Display];
    }

    pub struct Logger<'a> {
        parent: Option<&'a dyn Log>,
        prefix: &'a str,
        format: &'a str,
        arguments: &'a [&'a dyn Display],
    }

    impl Log for Logger<'_> {
        fn parent(&self) -> Option<&dyn Log> {
            self.parent
        }

        fn prefix(&self) -> &str {
            self.prefix
        }

        fn format(&self) -> &str {
            self.format
        }

        fn arguments(&self) -> &[&dyn Display] {
            self.arguments
        }
    }

    // This thread local storage allows preserving this thread's call stack while
    // being able to log from anywhere without the need to pass a logger around.
    //
    // Note that the `Lich<dyn Log>` can have the `'static` lifetime.
    thread_local! {
        static LOGGER: RefCell<Option<Lich<dyn Log>>> = RefCell::default();
    }

    pub fn scope<T: Display, F: FnOnce(&T)>(prefix: &str, argument: &T, function: F) {
        let parent = LOGGER.take();
        {
            // This `Logger` captures some references that live on the stack.
            let logger = Logger {
                parent: parent.as_deref(),
                prefix,
                format: "({})",
                arguments: &[argument],
            };
            // The `Soul` must be pinned since `Lich`es will refer to its memory.
            let soul = pin!(Soul::new(logger));
            // The `Lich` is bound to the `Soul` as a `dyn Trait` wrapper.
            let lich = soul.as_ref().bind::<dyn Log>();
            // Push this logger as the current scope.
            // The non-static `Logger` crosses a `'static` boundary.
            LOGGER.set(Some(lich));
            // Call the function.
            function(argument);
            // Pop the logger.
            LOGGER.take().expect("`Lich` has been pushed");
            // If a `Lich` bound to this `Soul` still lives at the time of drop,
            // `<Soul as Drop>::drop` will panic.
        }
        // Put back the old logger.
        LOGGER.set(parent);
    }
}

fn main() {
    #[cfg(all(feature = "cell", feature = "shroud"))]
    scoped_static_logger::scope("some-prefix", &37, |value| {
        assert_eq!(*value, 37);
    });
}
