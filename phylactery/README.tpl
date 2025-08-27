<div align="center"> <h1> {{ package.name }} {{ package.version }} </h1> </div>

<p align="center">
    <i> 
{{ package.description }}
    </i>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/{{ package.name }}/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/{{ package.name }}/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/{{ package.name }}"> <img src="https://img.shields.io/crates/v/{{ package.name }}.svg"> </a>
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
{% include "thread_spawn_bridge.rs" %}
```

###

<p align="right"><i> examples/scoped_static_logger.rs </i></p>

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