<div align="center"> <h1> phylactery 1.0.0 </h1> </div>

<p align="center">
    <i> 
Safe and thin wrappers around lifetime extension to allow non-static values to cross static boundaries.
    </i>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/phylactery/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/phylactery/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/phylactery"> <img src="https://img.shields.io/crates/v/phylactery.svg"> </a>
</div>

---
### In Brief
This library provides a way to extend the lifetime of a value beyond its original scope, allowing it to be used in `'static` contexts. It does this by splitting a pinned, owned value into two parts: a [`Soul`] and one or more [`Lich`]es.

- The [`Soul<T, B>`] owns the value `T` and controls its lifetime. It must be pinned in memory (e.g., on the stack with `core::pin::pin!` or on the heap with `Box::pin`).
- The [`Lich<T, B>`] is a handle to the value inside the `Soul`. It can be safely given a `'static` lifetime.
- The [`Binding`] `B` is a trait that defines the connection between the `Soul` and its `Lich`es, managing reference counting and access control.

When the `Soul` is dropped, it automatically severs the connection to all its `Lich`es, ensuring that the value can no longer be accessed. This makes it impossible to create a dangling reference.

The general usage pattern of this library is:
- Choose a `Soul` variant for your use-case (see below for the tradeoffs).
- Create a `Soul` with your data using `Soul::new(my_data)`.
- Pin the `Soul` to a stable memory location (e.g., `let soul = core::pin::pin!(Soul::new(my_data));`).
- Create one or more `Lich`es from the pinned `Soul` by calling `soul.as_ref().bind()`. The `Lich` can be shrouded as a `dyn Trait` object if needed.
- Use the `Lich` as a `'static` handle to your data.
- When the `Soul` is dropped, all `Lich`es bound to it are automatically and safely invalidated.

Different variants exist with different tradeoffs:
- [`phylactery::cell`]:
    - Based on `core::cell::Cell`.
    - Does **not** allocate heap memory for the binding (but the `Soul` can be heap-allocated with `Box::pin`).
    - If a `Lich` still exists when the `Soul` is dropped, the thread will panic.
    - `Lich` can be cloned.
    - Can **not** be sent to other threads.
    - Can be used with `#[no_std]`.
- [`phylactery::lock`]:
    - Based on `std::sync::RwLock` (or a spin-lock on `no_std`).
    - Does **not** allocate heap memory for the binding.
    - If a `Lich` still exists when the `Soul` is dropped, the thread will block until the `Lich` is dropped (which can lead to deadlocks).
    - `Lich` can be cloned.
    - Can be sent to other threads.
    
*Since this library makes use of some `unsafe` code, all tests are run with `miri` to try to catch any unsoundness.*

---
### Examples

<p align="right"><i> examples/thread_spawn_bridge.rs </i></p>

```rust
/// Trivially reimplement [`thread::scope`] in a more powerful way.
///
/// Contrary to other `scope` solutions, here, the captured reference can be
/// returned (as a [`Soul<P>`]) while the threads continue to execute.
#[cfg(all(feature = "lock", feature = "shroud"))]
pub mod thread_spawn_bridge {
    use core::{num::NonZeroUsize, pin::Pin};
    use phylactery::lock::Soul;
    use std::thread;

    pub fn broadcast<F: Fn(usize) + Send + Sync>(
        parallelism: NonZeroUsize,
        function: F,
    ) -> Pin<Box<Soul<F>>> {
        // Pin the `Soul` to the heap to be able to return it.
        let soul = Box::pin(Soul::new(function));
        // Spawn a bunch of threads that will all call `F`.
        for index in 0..parallelism.get() {
            // `Soul::bind` requires a pinning.
            let lich = soul.as_ref().bind::<dyn Fn(usize) + Send + Sync>();
            // The non-static function `F` crosses a `'static` boundary protected by the
            // `Lich` and is called on another thread. `Send/Sync` requirements still apply.
            thread::spawn(move || lich(index));
        }

        // The `Soul` continues to track the captured `F` and will guarantee that it
        // becomes inaccessible when it itself drops.
        //
        // If a `Lich` bound to this `Soul` still lives at the time of drop,
        // `<Soul as Drop>::drop` will block until all `Lich`es are dropped.
        soul
    }
}

fn main() {
    #[cfg(all(feature = "lock", feature = "shroud"))]
    thread_spawn_bridge::broadcast(
        std::thread::available_parallelism().unwrap_or(core::num::NonZeroUsize::MIN),
        &|index| println!("{index}"),
    );
}

```

###

<p align="right"><i> examples/scoped_static_logger.rs </i></p>

```rust
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

```

_See the [examples](examples/) and [tests](tests/) folder for more detailed examples._

---
### Contribute
- If you find a bug or have a feature request, please open an [issues](https://github.com/Magicolo/phylactery/issues).
- `phylactery` is actively maintained and [pull requests](https://github.com/Magicolo/phylactery/pulls) are welcome.
- If `phylactery` was useful to you, please consider leaving a [star](https://github.com/Magicolo/phylactery)!

---
