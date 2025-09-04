<div align="center"> <h1> phylactery 2.0.0 </h1> </div>

<p align="center">
    <i>
Crafted through a vile ritual, a phylactery is a magical receptacle that holds a necromancer's soul, permanently binding it to the mortal world as an immortal lich.
<br/><br/>
Safe and thin wrappers around lifetime extension to allow non-static values to cross static boundaries.
    </i>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/phylactery/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/phylactery/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/phylactery"> <img src="https://img.shields.io/crates/v/phylactery.svg"> </a>
</div>

---
### In Brief
- Wrap a value `T` with `Soul<T>::new(value)`.
- Pin the `Soul` with `core::pin::pin!` or `Box/Arc/Rc::pin`.
- Bind `Lich<dyn Trait>` to the `Soul` with `soul.bind::<dyn Trait>()` (where `Trait` is a trait implemented by `T`).
- Use the `Lich` in a lifetime-extended context (such as crossing a `std::thread::spawn` `'static` boundary or storing it in a `static` variable).
- Make sure to drop all `Lich`es before dropping the `Soul`.
- On drop, the `Soul` will block the thread until all `Lich`es are dropped, potentially creating a deadlock condition (in the name of memory safety).

*Since this library makes use of some `unsafe` code, all tests are run with `miri` to try to catch any unsoundness.*
*This library supports `#[no_std]` (use `default-features = false` in your 'Cargo.toml').*

---
### Examples

<p align="right"><i> examples/thread_spawn_bridge.rs </i></p>

```rust
/// Trivially reimplement [`thread::scope`] in a more powerful way.
///
/// Contrary to other `scope` solutions, here, the captured reference can be
/// returned (as a [`Soul<T>`]) while the threads continue to execute.
#[cfg(feature = "shroud")]
pub mod thread_spawn_bridge {
    use core::{num::NonZeroUsize, pin::Pin};
    use phylactery::Soul;
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
    #[cfg(feature = "shroud")]
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
#[cfg(feature = "shroud")]
pub mod scoped_static_logger {
    use core::{cell::RefCell, fmt::Display, pin::pin};
    use phylactery::{Lich, Soul, shroud};

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
    #[cfg(feature = "shroud")]
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
