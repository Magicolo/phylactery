# Issue 15: `Soul::sever` Contains Unreachable `panic!` With Misleading Message and Missing Documentation

## Summary

The doc comment on `Soul::sever` says:
> *Ensures that all bindings to this [`Soul`] are severed, blocking the current
> thread if any bound [`Lich`] remain **and returning the unpinned [`Soul`] on
> completion**.*

The function signature is:

```rust
pub fn sever<S: Deref<Target = Self>>(this: Pin<S>) -> S
```

It returns `S` (e.g., `Box<Soul<T>>`), not a `Soul<T>`.  This is accurate.

However, the related method `Soul::try_sever` has a doc comment that is
**correct** but the internal `// Safety:` comment on `Soul::unpin` references
`B::sever`, a symbol that does not exist anywhere in the codebase.  This is
documented in Issue 08.

This issue covers a different, related gap: `Soul::sever` has no documented
**panic condition**, even though it calls `panic!("sever failed possibly due to
unwinding")` in some paths.

## Location

`phylactery/src/soul.rs`, `Soul::sever`, lines 120-127.

```rust
pub fn sever<S: Deref<Target = Self>>(this: Pin<S>) -> S {
    if sever::<true>(&this.count) {
        // Safety: all bindings have been severed, guaranteed by `B::sever`.
        unsafe { Self::unpin(this) }
    } else {
        panic!("sever failed possibly due to unwinding")
    }
}
```

## Detailed Explanation

### What causes the panic?

`sever::<true>` can only return `false` if... looking at the implementation:

```rust
fn sever<const FORCE: bool>(count: &AtomicU32) -> bool {
    loop {
        match count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
            Ok(0 | u32::MAX) | Err(u32::MAX) => break true,
            Ok(value) | Err(value) if FORCE => atomic_wait::wait(count, value),
            Ok(_) | Err(_) => break false,
        }
    }
}
```

When `FORCE = true`, the only arm that can produce `false` is `Ok(_) | Err(_)`.
But that arm only fires when `FORCE = false`.  With `FORCE = true`, the loop
either breaks with `true` or blocks indefinitely.  Therefore `sever::<true>`
**never returns `false`**, and the `panic!` in `Soul::sever` is unreachable.

### The panic message is misleading

The message *"sever failed possibly due to unwinding"* suggests that stack
unwinding somehow causes `sever::<true>` to return `false`, but as shown above,
it can never return `false` when `FORCE = true`.  The message is both
unreachable and misleading.

### Documentation gap

The public documentation for `Soul::sever` should explicitly state:
- That it **blocks** the current thread until all Liches are dropped.
- That it **panics** if called during unwinding... wait, it doesn't.  So the
  doc should clarify it does NOT panic.
- The `#[must_use]` attribute is missing (see Issue 06).

## Plan to Fix

1. **Remove or replace the unreachable panic** with `unreachable!()` to make
   the logic clear to readers and to get a compiler warning if the branch ever
   becomes reachable:

```rust
pub fn sever<S: Deref<Target = Self>>(this: Pin<S>) -> S {
    if sever::<true>(&this.count) {
        // Safety: `sever::<true>` returned `true`, meaning all Liches have
        // been dropped and the count has been atomically set to u32::MAX.
        unsafe { Self::unpin(this) }
    } else {
        // `sever::<true>` loops until count == 0 and never returns false.
        unreachable!()
    }
}
```

2. **Add a `# Panics` section** to the doc comment that accurately reflects the
   panic behaviour (or lack thereof):

```rust
/// Ensures that all bindings to this [`Soul`] are severed, blocking the
/// current thread until all bound [`Lich`]es are dropped, then returns
/// the unpinned `S`.
///
/// # Panics
///
/// This method does not panic.  If blocking until all Liches are dropped
/// is not acceptable, use [`Soul::try_sever`] instead.
///
/// # Deadlock
///
/// If the calling thread holds the last [`Lich`] that keeps the count
/// non-zero, calling this method will deadlock.
pub fn sever<S: Deref<Target = Self>>(this: Pin<S>) -> S { … }
```

3. Run `cargo test` and `cargo clippy --all-features` to verify no regressions.
