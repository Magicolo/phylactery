<div align="center"> <h1> {{ package.name }} {{ package.version }} </h1> </div>

<p align="center">
    <i>
Crafted through a vile ritual, a phylactery is a magical receptacle that holds a necromancer's soul, permanently binding it to the mortal world as a lich.
<br/><br/>
{{ package.description }}
    </i>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/{{ package.name }}/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/{{ package.name }}/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/{{ package.name }}"> <img src="https://img.shields.io/crates/v/{{ package.name }}.svg"> </a>
</div>

---
### In Brief
- A `Soul<T, B>` wraps a given value `T`. When pinned (either with `core::pin::pin!` or `Box::pin`), it can produce `Lich<dyn Trait>`es that are bound to it (where `Trait` is a trait implemented by `T`). On drop, it will guarantee that the value `T` becomes unreachable (the behavior varies based on the `B: Binding`).
- A `Lich<T, B>` is a handle to the value inside the `Soul`. It may have any lifetime (including `'static`), thus it is allowed to cross `'static` boundaries (such as when `std::thread::spawn`ing a thread or when storing a value in a `static` variable).

Two `B: Binding` implementations are currently supported and offer different tradeoffs:
- `phylactery::cell::Cell`:
    - Uses a `core::cell::Cell<u32>` internally for reference counting.
    - Can **not** be sent to other threads.
    - Create a `Soul` using `phylactery::cell::Soul::new(..)`
    - When the `Soul` is dropped, the thread will panic unless all `Lich`es are dropped.
- `phylactery::atomic::Atomic`:
    - Uses a `core::sync::atomic::AtomicU32` for reference counting.
    - Can be sent to other threads.
    - Create a `Soul` using `phylactery::atomic::Soul::new(..)`
    - When the `Soul` is dropped, the thread will block until all `Lich`es are dropped.
    
*Since this library makes use of some `unsafe` code, all tests are run with `miri` to try to catch any unsoundness.*
*This library supports `#[no_std]` (use `default-features = false` in your 'Cargo.toml').*

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