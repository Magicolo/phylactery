use core::num::NonZeroUsize;
use std::thread::available_parallelism;

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
    // Note that the `Lich<dyn Log>` has the `'static` lifetime.
    thread_local! {
        static LOGGER: Cell<Option<Lich<dyn Log>>> = const { Cell::new(None) };
    }

    pub fn scope<T: Display, F: FnOnce(&T)>(prefix: &str, argument: &T, function: F) {
        let parent = LOGGER.take();
        {
            // `Lich::borrow` can fail if the binding between it and its `Soul<'a>` has been
            // severed.
            let guard = parent.as_ref().and_then(Lich::borrow);
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
            LOGGER.set(Some(lich));
            function(argument);
            // Pop the logger.
            let lich = LOGGER.take().expect("must get back our logger");
            // Although not strictly required in this case (letting the `Lich<T>` and
            // `Soul<'a>` be dropped would also work), `redeem` is the recommended
            // pattern to dispose of a `Lich<T>` and `Soul<'a>` pair since it is going to
            // work with all variants of `Lich<T>/Soul<'a>`.
            redeem(lich, soul).expect("must be able to redeem");
        }
        // Put back the old logger.
        LOGGER.set(parent);
    }
}

/// Trivially reimplement `thread::scope` in a more powerful way.
#[cfg(feature = "lock")]
#[allow(clippy::manual_try_fold)]
pub mod thread_spawn_bridge {
    use core::num::NonZeroUsize;
    use phylactery::lock::{redeem, ritual};
    use std::thread;

    pub fn broadcast<F: Fn(usize) + Send + Sync>(parallelism: NonZeroUsize, function: F) {
        // `Shroud` is already implemented for all `Fn(..) -> T`, `FnMut(..) -> T` and
        // `FnOnce(..) -> T` with all of their `Send`, `Sync` and `Unpin` permutations.
        let (lich, soul) = ritual::<_, dyn Fn(usize) + Send + Sync>(&function);
        // Spawn a bunch of threads that will all call `F` and collect their
        // `JoinHandle`.
        let handles = (0..parallelism.get())
            .map(|index| {
                let lich = lich.clone();
                // The non-static function `F` will cross a `'static` boundary wrapped within
                // the `Lich<T>`.
                thread::spawn(move || {
                    let lich = lich;
                    {
                        let guard = lich
                            .borrow()
                            .expect("since the `Soul<'a>` still lives, this must succeed");
                        // Call the non-static function.
                        guard(index);
                    }
                    lich
                })
            })
            .collect::<Vec<_>>();

        // `redeem` all `Lich<T>`es with their `Soul<'a>`.
        let soul = handles.into_iter().fold(soul, |soul, handle| {
            let lich = handle.join().expect("thread succeeded");
            // `redeem` will give back the `Soul<'a>` if more `Lich<T>` exist
            redeem(lich, soul)
                .expect("must be able to redeem")
                .expect("must be `Some` since some `Lich<T>` remain")
        });

        // All `Lich<T>`es have been `redeem`ed, so the `Soul<'a>` must be `None`.
        assert!(
            redeem(lich, soul)
                .expect("must be able to redeem")
                .is_none()
        );
    }
}

fn main() {
    scoped_static_logger::scope("some-prefix", &37, |value| {
        assert_eq!(*value, 37);
    });
    thread_spawn_bridge::broadcast(
        available_parallelism().unwrap_or(NonZeroUsize::MIN),
        |index| println!("{index}"),
    );
}
