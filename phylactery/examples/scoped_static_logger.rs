/// Implements a thread local scoped logger available from anywhere that can
/// borrow values that live on the stack.
#[cfg(all(feature = "cell", feature = "shroud"))]
pub mod scoped_static_logger {
    use core::{cell::RefCell, fmt::Display};
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
        // This `Logger` captures some references that live on the stack.
        let logger = Logger {
            parent: parent.as_deref(),
            prefix,
            format: "({})",
            arguments: &[argument],
        };
        // Providing a memory location for the `Soul`'s reference count relieves the
        // need to allocate it to the heap.
        let mut count = 0;
        let soul = Soul::new_with(&logger, &mut count);
        let lich = soul.bind::<dyn Log>();
        // Push this logger as the current scope.
        LOGGER.set(Some(lich));
        function(argument);
        // Pop the logger.
        let lich = LOGGER.take().expect("`Lich` has been pushed");
        // Although not strictly required (letting the `Lich` be dropped would also
        // work), `redeem` is the recommended pattern to dispose of a `Lich`.
        soul.redeem(lich)
            .ok()
            .expect("`Lich` has been bound by this `Soul`");
        // If a `Lich` bound to this `Soul` still lives at the time of drop,
        // `<Soul as Drop>::drop` will panic.
        drop(soul);
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
