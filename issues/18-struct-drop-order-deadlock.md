# Issue 18: Deadlock When Soul and Lich Are Stored in the Same Struct

## Summary

Rust drops struct fields in **declaration order** (first declared, first
dropped).  If a `Soul<T>` is declared **before** the `Lich`(es) bound to it in
the same struct, the `Soul` is dropped first.  `<Soul as Drop>::drop` calls
`sever`, which blocks waiting for all `Lich`es to drop.  But the `Lich`es have
not been dropped yet (they are dropped after the `Soul`).  The result is an
**unconditional deadlock** on the thread running the struct's destructor.

This is a known footgun that is not clearly documented and has no mitigation
tooling in the library.

## Why This Happens (Rust Drop Order)

Rust specifies:
- **Local variables**: dropped in **reverse** declaration order.
- **Struct fields**: dropped in **forward** (declaration) order.

This asymmetry means the "natural" pattern for local variables (Soul before
Lich) is safe, but the same layout in a struct is deadly.

```rust
// OK in a function:
let soul = Box::pin(Soul::new(|| {}));
let lich = soul.as_ref().bind::<dyn Fn()>();
// drop order: lich, then soul  ✓

// DEADLOCK in a struct:
struct State {
    soul: Pin<Box<Soul<fn()>>>,  // dropped 1st
    lich: Lich<dyn Fn()>,       // dropped 2nd
}
// soul.drop() blocks waiting for lich, but lich hasn't dropped yet → DEADLOCK
```

## Affected Code Paths

This affects any struct where:
1. A `Pin<Box<Soul<T>>>` (or `Pin<Arc<Soul<T>>>` / `Pin<Rc<Soul<T>>>`) is a
   field AND
2. One or more `Lich<S>` bound to the same Soul are also fields, AND
3. The Soul is declared before the Liches in the struct definition.

## Why This Is an Issue

### Inadequate documentation

The current `Soul` documentation says:
> *If a Soul is dropped while any of its Liches are still alive, the drop
> implementation will block the current thread until all Liches are dropped.*

And the README says:
> *Make sure to drop all Liches before dropping the Soul.*

Neither the type documentation nor the README explicitly warns about the struct
drop order hazard, which is the most common way a developer might inadvertently
violate the "drop Liches first" requirement.

### No compile-time or run-time protection

The library currently provides no mechanism to:
- Warn at compile time if a Soul and its Liches coexist in the same struct in
  the wrong order.
- Detect at runtime (even in debug builds) that a Soul is blocked because its
  own Liches have not yet been dropped.
- Provide a `#[must_use]` or `#[must_not_hold]`-style lint.

### The same issue arises with `Arc::pin`

```rust
struct SharedState {
    soul: Pin<Arc<Soul<fn()>>>,
    lich: Lich<dyn Fn()>,
}
// When the Arc<Soul> reference count drops to zero (after soul's clone is
// released), soul blocks waiting for lich — which might never drop if lich
// holds the last Arc reference to SharedState.  Deadlock.
```

## Plan to Fix

### Documentation (minimal, non-breaking)

1. Add a **`# Deadlock` section** to `Soul`'s type-level documentation:

```rust
/// # Deadlock Hazard
///
/// When `Soul` and its [`Lich`]es are stored in the same struct, the struct's
/// fields must be declared so that all [`Lich`]es appear **before** the `Soul`.
/// Rust drops struct fields in declaration order, so declaring the `Soul` first
/// will cause its drop to block waiting for the `Lich`es, which have not yet
/// dropped — resulting in a deadlock.
///
/// ```rust
/// # use phylactery::{Soul, Lich};
/// # use core::pin::Pin;
/// // CORRECT: Lich before Soul
/// struct Correct {
///     lich: Lich<dyn Fn()>,  // dropped first ✓
///     soul: Pin<Box<Soul<fn()>>>,  // dropped second ✓
/// }
///
/// // INCORRECT: Soul before Lich → DEADLOCK
/// struct Wrong {
///     soul: Pin<Box<Soul<fn()>>>,  // would block waiting for lich
///     lich: Lich<dyn Fn()>,       // never reached
/// }
/// ```
```

2. Add the same warning to the crate-level README under a "Pitfalls" section.

### Optional: Runtime detection (debug builds)

Add a `debug_assert` in `<Soul as Drop>::drop` that fires if the blocking wait
takes longer than a threshold, printing a diagnostic message:

```rust
impl<T: ?Sized> Drop for Soul<T> {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();
        sever::<true>(&self.count);
        #[cfg(debug_assertions)]
        if start.elapsed() > std::time::Duration::from_millis(100) {
            eprintln!(
                "phylactery: Soul blocked for {:?} during drop. \
                 Ensure all Liches are dropped before the Soul. \
                 Check struct field declaration order.",
                start.elapsed()
            );
        }
    }
}
```

This is opt-in and only active in debug builds.

### Optional: Compile-time lint (nightly only)

A future version could use `#[must_not_hold]` (an unstable attribute) to warn
when a `Soul` and a `Lich` bound to it are held together, but this is currently
not feasible on stable Rust.
