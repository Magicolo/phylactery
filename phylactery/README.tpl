<div align="center"> <h1> {{ package.name }} {{ package.version }} </h1> </div>

<p align="center">
    <i>
Crafted through a vile ritual, a phylactery is a magical receptacle that holds a necromancer's soul, permanently binding it to the mortal world as an immortal lich.
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
- Wrap a value `T` with `Soul<T>::new(value)`.
- Pin the `Soul` with `core::pin::pin!` or `Box/Arc/Rc::pin`.
- Bind `Lich<dyn Trait>` to the `Soul` with `soul.bind::<dyn Trait>()` (where `Trait` is a trait implemented by `T`).
- Use the `Lich` in a lifetime-extended context (such as crossing a `std::thread::spawn` `'static` boundary or storing in a `static` variable).
- Make sure to drop all `Lich`es before dropping the `Soul`.
- On drop, the `Soul` will block the thread until all `Lich`es are dropped, potentially creating a deadlock condition (in the name of memory safety).

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