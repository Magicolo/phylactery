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

The general usafe pattern of this library is:
- Choose a `Lich<T>/Soul<'a>` variant for you use case (see below for the tradeoffs).
- Implement `Shroud` for the trait for which you want to extend the lifetime (a simple call to `shroud!(Trait)` is often all it takes).
- Use the corresponding `ritual::<T: Trait, dyn Trait>(value: &'a T)` to produce a `Lich<dyn Trait + 'static>` bound to a `Soul<'a>`.
- Use the `Lich<dyn Trait>` as a `'static` reference to your otherwise non-static `&'a T`.
- Use the corresponding `redeem(Lich<T>, Soul<'a>)` to guarantee that all references to `&'a T` are dropped before the end of lifetime `'a`.

When `Soul<'a>` is dropped or when calling `Soul::sever`, it is guaranteed that the captured reference is also dropped, thus
inaccessible from a remaining `Lich<T>`.

Different variants exist with different tradeoffs:
- `phylactery::raw`: 
    - Is as lightweight as a new type around a pointer (no allocation).
    - Does require the `Lich<T>` to be `redeem`ed (otherwise, `Lich<T>` and `Soul<'a>` **will** panic on drop).
    - Does require some `unsafe` calls.
    - `Lich<T>` can **not** be cloned.
    - Can be sent to other threads.
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
{% include "cheat.rs" %}
```

_See the [examples](examples/) and [tests](tests/) folder for more detailed examples._

---
### Contribute
- If you find a bug or have a feature request, please open an [issues](https://github.com/Magicolo/{{ package.name }}/issues).
- `{{ package.name }}` is actively maintained and [pull requests](https://github.com/Magicolo/{{ package.name }}/pulls) are welcome.
- If `{{ package.name }}` was useful to you, please consider leaving a [star](https://github.com/Magicolo/{{ package.name }})!

---