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
Given a trait `Trait` and a `T: Trait`, any `&'a T` can be split into a `Lich<dyn Trait + 'static>` and a `Soul<'a>` pair such that the `dyn Trait` can cross `'static` boundaries while tracking the lifetime `'a`.

The general usage pattern of this library is:
- Choose a `Lich`/`Soul` variant for your use-case (see below for the tradeoffs).
- Implement `Shroud` for the trait for which you want to extend the lifetime (e.g. `#[shroud] trait Trait` will `impl<T: Trait> Shroud<T> for dyn Trait` automatically).
- Use the corresponding `ritual::<T: Trait, dyn Trait>(value: &'a T)` to produce a `Lich<dyn Trait + 'static>` bound to a `Soul<'a>`.
- Use the `Lich<dyn Trait>` as a `'static` reference to your otherwise non-`'static` `&'a T`.
- Use the corresponding `redeem(Lich<T>, Soul<'a>)` to guarantee that all references to `&'a T` are dropped before the end of lifetime `'a`.

When `Soul<'a>` is dropped or when calling `Soul::sever`, it is guaranteed that the captured reference is also dropped, thus inaccessible from a remaining `Lich`.

Different variants exist with different tradeoffs:
- `phylactery::raw`:
    - Zero-cost (wraps a pointer in a new-type).
    - Does **not** allocate heap memory.
    - Does require the `Lich` to be `redeem`ed with its `Soul` (otherwise, `Lich` and `Soul` **will** panic on drop).
    - Does require some `unsafe` calls (e.g. `Lich::borrow`).
    - `Lich` can **not** be cloned.
    - Can be sent to other threads.
    - Can be used with `#[no_std]`.
- `phylactery::atomic`:
    - Adds minimal overhead with an `AtomicU32` reference counter.
    - Does **not** allocate heap memory.
    - Does require an additional memory location (an `&mut u32`) to create the `Lich`/`Soul` pair.
    - If a `Lich` still exists when the `Soul` is dropped, the thread will block until the `Lich` is dropped (which can lead to deadlocks).
    - Does **not** require `unsafe` calls.
    - `Lich` can be cloned.
    - Can be sent to other threads.
    - Can be used with `#[no_std]`.
- `phylactery::cell`:
    - Adds an indirection and minimal overhead using `Rc<RefCell>`.
    - Does allocate heap memory.
    - Allows for the use of the `Lich::sever` and `Soul::sever` methods.
    - If a borrow still exists when the `Soul` is dropped, the thread will panic.
    - Does **not** require the `Lich`es to be `redeem`ed (although it is considered good practice to do so).
    - Does **not** require `unsafe` calls.
    - `Lich` can be cloned.
    - Can **not** be sent to other threads.
- `phylactery::lock`:
    - Adds an indirection and *some* overhead using `Arc<RwLock>`.
    - Does allocate heap memory.
    - Allows for the use of the `Lich::sever` and `Soul::sever` methods.
    - If a borrow still exists when the `Soul` is dropped, the thread will block until the borrow expires (which can lead to deadlocks).
    - Does **not** require the `Lich` to be `redeem`ed (although it is considered good practice to do so).
    - Does **not** require `unsafe` calls.
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
