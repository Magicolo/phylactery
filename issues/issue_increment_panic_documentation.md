# Issue: `increment` Panics on Maximum `Lich` Count — Should Use `Result` or Document Limit

## Summary

The `increment` function in `phylactery/src/lich.rs` panics with `"maximum number of Liches reached"` when the reference count reaches `u32::MAX - 1` (4,294,967,294). While this limit is enormous and unlikely to be hit in practice, the panic occurs inside `Soul::bind()` and `Lich::clone()`, which are public API methods. Panicking in these methods is surprising behavior that should either be documented or converted to a `Result` return type.

## Why This Is an Issue

Current code:

```rust
// phylactery/src/lich.rs, line 181-197
pub(crate) fn increment(count: &AtomicU32) -> u32 {
    let result = count.fetch_update(Ordering::Acquire, Ordering::Relaxed, |value| {
        if value < SEVERED - 1 {
            Some(value + 1)
        } else {
            None
        }
    });
    match result {
        Ok(value) => value,
        Err(SEVERED) => unreachable!("bind called on a severed Soul"),
        Err(_) => panic!("maximum number of `Lich`es reached"),
    }
}
```

This is called from:
1. `Soul::bind()` — a public method that creates a new `Lich`.
2. `Lich::clone()` — a public method that clones an existing `Lich`.

Neither method's documentation mentions that it can panic on overflow.

### The standard library's approach

For comparison, `Arc::clone()` also panics on reference count overflow, but documents it:

> This function will panic if the reference count overflows.

Similarly, `Rc::clone()` panics on overflow.

So the panic behavior is consistent with standard library conventions, but it should be documented.

## Impact

- **Severity**: Very Low (documentation)
- **Likelihood**: Essentially zero in practice (need 4 billion Liches).
- **The panic is correct behavior** — allowing overflow would be unsound (count wraps to 0 or SEVERED, causing premature sever).

## Proposed Fix

### Option A: Document the panic (minimal change)

Add a `# Panics` section to `Soul::bind()`:

```rust
/// # Panics
///
/// Panics if the number of bound [`Lich`]es reaches `u32::MAX - 1`
/// (approximately 4 billion). This limit is consistent with the
/// behavior of [`Arc::clone`](std::sync::Arc::clone).
pub fn bind<S: Shroud<T> + ?Sized>(self: Pin<&Self>) -> Lich<S> { ... }
```

And to `Lich::clone()`:

```rust
impl<T: ?Sized> Clone for Lich<T> {
    /// # Panics
    ///
    /// Panics if the number of bound [`Lich`]es reaches `u32::MAX - 1`.
    fn clone(&self) -> Self { ... }
}
```

### Option B: Return `Result` from `bind` (breaking change)

This is more disruptive but more Rust-idiomatic:

```rust
pub fn try_bind<S: Shroud<T> + ?Sized>(self: Pin<&Self>) -> Result<Lich<S>, BindError> { ... }
```

However, since `Arc::clone()` also panics on overflow, the current behavior is considered acceptable.

## Files to Modify

- `phylactery/src/soul.rs` (line 88-100): Add `# Panics` documentation to `bind`.
- `phylactery/src/lich.rs` (line 96-104): Add `# Panics` documentation to `clone`.

## Verification

- Documentation renders correctly: `cargo doc --all-features --no-deps`.

## Notes

The existing test `too_many_liches_panics` is commented out (lines 181-189 in `binding.rs`) because it's "too slow to run" (iterating 4 billion times). This is expected — the limit is high enough that it shouldn't be hit in practice.
