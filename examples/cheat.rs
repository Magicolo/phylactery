/// Trivially reimplement `thread::scope` in a more powerful way.
/// Contrarily to other `scope` solutions, here, the captured reference can be
/// returned (as a `Soul<'a>`) while the threads continue to execute.
#[cfg(feature = "lock")]
pub mod thread_spawn_bridge {
    use core::num::NonZeroUsize;
    use phylactery::lock::{Soul, ritual};
    use std::thread;

    pub fn broadcast<F: Fn(usize) + Send + Sync>(
        parallelism: NonZeroUsize,
        function: &F,
    ) -> Soul<'_> {
        // `Shroud<F>` is already implemented for all `Fn(..) -> T`, `FnMut(..) -> T`
        // and `FnOnce(..) -> T` with all of their `Send`, `Sync` and `Unpin`
        // permutations.
        let (lich, soul) = ritual::<_, dyn Fn(usize) + Send + Sync>(function);
        // Spawn a bunch of threads that will all call `F`.
        for index in 0..parallelism.get() {
            let lich = lich.clone();
            // The non-static function `F` crosses a `'static` boundary protected by the
            // `Lich<T>`.
            thread::spawn(move || {
                // Borrowing may fail if the `Soul<'a>` has been dropped/severed.
                if let Some(guard) = lich.borrow() {
                    // Call the non-static function.
                    guard(index);
                }
                // Allow the `Guard` and `Lich<T>` to drop such that the
                // `Soul<'a>` can complete its `Soul::sever`.
            });
        }

        // The `Soul<'a>` continues to track the captured `'a` reference and will
        // guarantee that it becomes inaccessible when it itself drops.
        // Note that this may block this thread if there still are active borrows at the
        // time of drop.
        //
        // Note that the `Lich<T>`es do not need be `redeem`ed.
        soul
    }
}

/// Have a thread local scoped logger available from anywhere that can borrow
/// values that live on the stack.
#[cfg(feature = "cell")]
pub mod scoped_static_logger {
    use core::{cell::Cell, fmt::Display};
    // Uses the `cell` variant; see `lock` for a thread-safe version or `raw` for a even more
    // lightweight version (with some additional safety burden).
    use phylactery::{
        cell::{Lich, redeem, ritual},
        shroud,
    };

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

    // Use the convenience macro to automatically implement the required `Shroud`
    // trait for all `T: Log`.
    shroud!(Log);

    // This thread local storage allows preserve this thread's call stack while
    // being able to log from anywhere without the need to pass a logger around.
    //
    // Note that the `Lich<dyn Log>` implements `Default` and has the `'static`
    // lifetime.
    thread_local! {
        static LOGGER: Cell<Lich<dyn Log>> = Cell::default();
    }

    pub fn scope<T: Display, F: FnOnce(&T)>(prefix: &str, argument: &T, function: F) {
        let parent = LOGGER.take();
        {
            // `Lich::borrow` can fail if the binding between it and its `Soul<'a>` has been
            // severed.
            let guard = parent.borrow();
            // This `Logger` captures some references that live on the stack.
            let logger = Logger {
                parent: guard.as_deref(),
                prefix,
                format: "({})",
                arguments: &[argument],
            };
            // `ritual` produces a `Lich<dyn Log + 'static>` and `Soul<'a>` pair.
            let (lich, soul) = ritual::<_, dyn Log + 'static>(&logger);
            // Push this logger as the current scope.
            LOGGER.set(lich);
            function(argument);
            // Pop the logger.
            let lich = LOGGER.take();
            // Although not strictly required in this case (letting the `Lich<T>` and
            // `Soul<'a>` be dropped would also work), `redeem` is the recommended
            // pattern to dispose of a `Lich<T>` and `Soul<'a>` pair since it is going to
            // work with all variants of `Lich<T>/Soul<'a>`.
            redeem(lich, soul).ok().expect("must be able to redeem");
        }
        // Put back the old logger.
        LOGGER.set(parent);
    }
}

fn main() {
    #[cfg(feature = "cell")]
    scoped_static_logger::scope("some-prefix", &37, |value| {
        assert_eq!(*value, 37);
    });

    #[cfg(feature = "lock")]
    thread_spawn_bridge::broadcast(
        std::thread::available_parallelism().unwrap_or(core::num::NonZeroUsize::MIN),
        &|index| println!("{index}"),
    );
}
