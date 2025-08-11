#![cfg(feature = "cell")]
//! Implements a thread local scoped logger available from anywhere that can
//! borrow values that live on the stack.

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

fn main() {
    scope("some-prefix", &37, |value| {
        assert_eq!(*value, 37);
    });
}
