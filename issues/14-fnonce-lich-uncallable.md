# Issue 14: `Lich` Dereferencing a `dyn FnOnce` Is a Soundness / Usability Trap

## Summary

The `shroud_fn!` macro in `phylactery/src/shroud.rs` generates `Shroud`
implementations for `dyn FnOnce(…) -> R` (and its `Send`/`Sync`/`Unpin`
variants).  This allows creating `Lich<dyn FnOnce(…) -> R>`.  However,
`dyn FnOnce` behind a shared reference (`&dyn FnOnce`) cannot be called —
`FnOnce::call_once` consumes `self`, which is impossible through a `&`
reference.  The generated impl compiles but the resulting `Lich` is
**uncallable**, and calling it requires going through additional indirection
(e.g., `Pin<&mut dyn FnOnce>`), which is not the expected ergonomic use of a
`Lich`.

## Location

`phylactery/src/shroud.rs`, lines 192-194:

```rust
shroud_fn!(Fn(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
shroud_fn!(FnMut(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
shroud_fn!(FnOnce(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
```

## Detailed Explanation

### Why `Lich<dyn FnOnce(…) -> R>` cannot be called

`Lich<T>` implements `Deref<Target = T>`, providing a `&T`.  For `T = dyn
FnOnce(A) -> R`:
- `FnOnce::call_once(self, args)` requires *ownership* of the receiver.
- You cannot own a `dyn FnOnce` through `&dyn FnOnce` — you only have a shared
  reference.
- Rust's standard library does **not** implement `FnOnce` for `&dyn FnOnce`
  (it does implement `FnMut`/`Fn` for `&mut dyn FnMut` / `&dyn Fn`).

Therefore:

```rust
let soul = Box::pin(Soul::new(|| 42u32));
let lich: Lich<dyn FnOnce() -> u32> = soul.as_ref().bind::<dyn FnOnce() -> u32>();
let result = (*lich)(); // compile error: cannot call `dyn FnOnce()` by value
```

### Confirmed reproduction

The compile error has been confirmed.  A compile-fail doc test
`can_not_call_lich_dyn_fnonce` was added to `phylactery/src/lib.rs` under the
`fails` module.  It verifies that `(*lich)()` is rejected by the compiler:

```bash
cargo test --doc --features shroud
# test phylactery/src/lib.rs - fails::can_not_call_lich_dyn_fnonce … ok
```

See `phylactery/examples/issue_14_fnonce_uncallable.rs` for a runnable example
that demonstrates the usability trap (creating but not calling the Lich).

The only ways to call `dyn FnOnce` are:
- Through `Box<dyn FnOnce>` (`Box::call_once`)
- Manually through a raw pointer

Neither is available from a `Lich`.

### Contrast with `Fn` and `FnMut`

| Trait     | Lich callable?     | Reason                                                 |
|-----------|--------------------|--------------------------------------------------------|
| `Fn`      | ✓ Yes              | `Fn` is implemented for `&dyn Fn` (Rust built-in)     |
| `FnMut`   | ✗ No (without &mut)| `FnMut` is implemented for `&mut dyn FnMut` only      |
| `FnOnce`  | ✗ No               | No impl of `FnOnce` for `&dyn FnOnce`                  |

`FnMut` through a `Lich` requires `Pin<&mut>` or `RefCell` indirection; it
works but is awkward.  `FnOnce` doesn't work at all through an immutable
reference.

### The same `dyn FnMut` issue

`Lich<dyn FnMut(…) -> R>` is also not directly callable because `Deref` gives
`&T`, not `&mut T`.  A user needs `RefCell<dyn FnMut>` or similar interior
mutability to call through a `Lich`.

## Why This Is an Issue

1. **Misleading API surface:** Generating `Shroud` impls for `FnOnce` implies
   that `Lich<dyn FnOnce>` is a supported and useful type, which it is not in
   practice.
2. **Potential misuse:** A user might write `Lich<dyn FnOnce()>` expecting to
   be able to call it, and only discover the limitation after a confusing
   compiler error.
3. **Documentation gap:** Neither the `Lich` type nor the `shroud_fn!` macro
   documentation warns about this limitation.

## Plan to Fix

### Option A: Remove `shroud_fn!` for `FnOnce`

Since `Lich<dyn FnOnce>` is not meaningfully callable, remove the `FnOnce`
expansion:

```rust
shroud_fn!(Fn(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
shroud_fn!(FnMut(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
// FnOnce removed — not usable through a shared reference
```

This is a breaking change if any downstream crate uses `Lich<dyn FnOnce>`, so
it requires a major version bump.

### Option B: Document the limitation (non-breaking)

Keep the `FnOnce` impl but add a prominent note in the crate documentation and
in the `Shroud` trait documentation:

> **Note for `FnOnce`:** A `Lich<dyn FnOnce(…) -> R>` cannot be called
> directly through `Deref` because calling `FnOnce` requires consuming the
> receiver.  If you need a one-shot callable, consider wrapping the closure in
> a `Mutex<Option<Box<dyn FnOnce>>>` and using `Lich<dyn Fn(…)>` to call it.

### Option C: Add a `call_once` method to `Lich` for `FnOnce`

```rust
impl<A: Tuple, R> Lich<dyn FnOnce<A, Output = R>> {
    /// Calls the underlying `FnOnce`, consuming this `Lich`.
    /// # Panics
    /// Panics if called more than once (this is statically enforced by
    /// consuming `self`).
    pub fn call_once(self, args: A) -> R {
        // Safety: we consume `self`, ensuring this is called at most once.
        // The Soul guarantees the value remains valid.
        unsafe { (self.value.as_ptr() as *mut dyn FnOnce<A, Output = R>).call_once(args) }
    }
}
```

This requires nightly (`FnOnce` via raw pointer dispatch) and is complex.
Option B is recommended for the near term.
