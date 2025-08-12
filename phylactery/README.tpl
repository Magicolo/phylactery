<div align="center"> <h1> {{ package.name }} {{ package.version }} </h1> </div>

<p align="center">
    <em> 
{{ package.description }}
    </em>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/{{ package.name }}/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/{{ package.name }}/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/{{ package.name }}"> <img src="https://img.shields.io/crates/v/{{ package.name }}.svg"> </a>
</div>

---
### In Brief
Given a trait `Trait` and a `T: Trait`, any `&'a T` can be split into a `Lich<dyn Trait + 'static>` and a `Soul<'a>` pair such that the `dyn Trait` can cross `'static` boundaries while tracking the lifetime `'a`.

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
    - Does **not** allocate heap memory.
    - Does require the `Lich<T>` to be `redeem`ed with its `Soul<'a>` (otherwise, `Lich<T>` and `Soul<'a>` **will** panic on drop).
    - Does require some `unsafe` calls (`Lich<T>::borrow`).
    - `Lich<T>` can **not** be cloned.
    - Can be sent to other threads.
    - Can be used with `#[no_std]`.
- `phylactery::atomic`:
    - Adds minimal overhead with an `AtomicU32` reference counter.
    - Does **not** allocate heap memory.
    - Does require an additional memory location (an `&mut u32`) to create the `Lich<T>/Soul<'a>` pair.
    - If a `Lich<T>` still exists when the `Soul<'a>` is dropped, the thread will block until the `Lich<T>` is dropped (which can lead to dead locks).
    - Does **not** require `unsafe` calls.
    - `Lich<T>` can be cloned.
    - Can be sent to other threads.
    - Can be used with `#[no_std]`.
- `phylactery::cell`:
    - Adds an indirection and minimal overhead using `Rc<RefCell>`.
    - Does allocate heap memory.
    - Allows for the use of the `Lich<T>/Soul<'a>::sever` methods.
    - If a borrow still exists when the `Soul<'a>` is dropped, the thread will panic.
    - Does **not** require the `Lich<T>`es to be `redeem`ed (although it is considered good practice to do so).
    - Does **not** require `unsafe` calls.
    - `Lich<T>` can be cloned.
    - Can **not** be sent to other threads.
- `phylactery::lock`:
    - Adds an indirection and *some* overhead using `Arc<RwLock>`.
    - Does allocate heap memory.
    - Allows for the use of the `Lich<T>/Soul<'a>::sever` methods.
    - If a borrow still exists when the `Soul<'a>` is dropped, the thread will block until the borrow expires (which can lead to dead locks).
    - Does **not** require the `Lich<T>` to be `redeem`ed (although it is considered good practice to do so).
    - Does **not** require `unsafe` calls.
    - `Lich<T>` can be cloned.
    - Can be sent to other threads.
    
*Since this library makes use of some `unsafe` code, all tests are run with `miri` to try to catch any unsoundness.*

---
### Examples

<p align="right"><em> examples/thread_spawn_bridge.rs </em></p>

```rust
{% include "thread_spawn_bridge.rs" %}
```

###

<p align="right"><em> examples/scoped_static_logger.rs </em></p>

```rust
{% include "scoped_static_logger.rs" %}
```

_See the [examples](examples/) and [tests](tests/) folder for more detailed examples._

---
### Contribute
- If you find a bug or have a feature request, please open an [issues](https://github.com/Magicolo/{{ package.name }}/issues).
- `{{ package.name }}` is actively maintained and [pull requests](https://github.com/Magicolo/{{ package.name }}/pulls) are welcome.
- If `{{ package.name }}` was useful to you, please consider leaving a [star](https://github.com/Magicolo/{{ package.name }})!

---