<div align="center"> <h1> phylactery 0.3.3 </h1> </div>

<p align="center">
    <em> 
Safe and thin wrappers around lifetime extension to allow non-static values to cross static boundaries.

Given a trait `Trait` and a `T: Trait`, any `&'a T` can be split into a `Lich<dyn Trait + 'static>` and a `Soul<'a>` pair such that the `dyn Trait` can cross `'static` boundaries while tracking the lifetime `'a`.
    </em>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/phylactery/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/phylactery/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/phylactery"> <img src="https://img.shields.io/crates/v/phylactery.svg"> </a>
</div>

---
### In Brief

The general usage pattern of this library is:
- Choose a `Lich<T>/Soul<'a>` variant for your use-case (see below for the tradeoffs).
- Implement `Shroud` for the trait for which you want to extend the lifetime (a simple call to `shroud!(Trait)` is often all it takes).
- Use the corresponding `ritual::<T: Trait, dyn Trait>(value: &'a T)` to produce a `Lich<dyn Trait + 'static>` bound to a `Soul<'a>`.
- Use the `Lich<dyn Trait>` as a `'static` reference to your otherwise non-static `&'a T`.
- Use the corresponding `redeem(Lich<T>, Soul<'a>)` to guarantee that all references to `&'a T` are dropped before the end of lifetime `'a`.

When `Soul<'a>` is dropped or when calling `Soul::sever`, it is guaranteed that the captured reference is also dropped, thus
inaccessible from a remaining `Lich<T>`.

Different variants exist with different tradeoffs:
- `phylactery::raw`: 
    - Zero cost (wraps a pointer in a new type).
    - Does require the `Lich<T>` to be `redeem`ed with its `Soul<'a>` (otherwise, `Lich<T>` and `Soul<'a>` **will** panic on drop).
    - Does require some `unsafe` calls (`Lich<T>::borrow`).
    - `Lich<T>` can **not** be cloned.
    - Can be sent to other threads.
    - Can be used in `#[no_std]` contexts.
- `phylactery::cell`: 
    - Adds an indirection and minimal overhead using `Rc<RefCell>`.
    - Allows for the use of the `Lich<T>/Soul<'a>::sever` methods.
    - If a borrow still exists when the `Soul<'a>` is dropped, the thread will panic.
    - Does **not** require the `Lich<T>`es to be `redeem`ed (although it is considered good practice to do so).
    - Does **not** require `unsafe` calls.
    - `Lich<T>` can be cloned.
    - Can **not** be sent to other threads.
- `phylactery::lock`:
    - Adds an indirection and *some* overhead using `Arc<RwLock>`.
    - Allows for the use of the `Lich<T>/Soul<'a>::sever` methods.
    - If a borrow still exists when the `Soul<'a>` is dropped, the thread will block until the borrow expires (which can lead to dead locks).
    - Does **not** require the `Lich<T>` to be `redeem`ed (although it is considered good practice to do so).
    - Does **not** require `unsafe` calls.
    - `Lich<T>` can be cloned.
    - Can be sent to other threads.
    
*Since this library makes use of some `unsafe` code, all tests are run with `miri` to try to catch any unsoundness.*

---
### Cheat Sheet

```rust
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
                .ok()
                .expect("must be able to redeem")
                .expect("must be `Some` since some `Lich<T>` remain")
        });

        // All `Lich<T>`es have been `redeem`ed, so the `Soul<'a>` must be `None`.
        assert!(
            redeem(lich, soul)
                .ok()
                .expect("must be able to redeem")
                .is_none()
        );
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
        |index| println!("{index}"),
    );
}

```

_See the [examples](examples/) and [tests](tests/) folder for more detailed examples._

---
### Contribute
- If you find a bug or have a feature request, please open an [issues](https://github.com/Magicolo/phylactery/issues).
- `phylactery` is actively maintained and [pull requests](https://github.com/Magicolo/phylactery/pulls) are welcome.
- If `phylactery` was useful to you, please consider leaving a [star](https://github.com/Magicolo/phylactery)!

---
