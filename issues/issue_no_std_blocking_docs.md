# Issue: Missing Documentation for `no_std` Blocking Behavior

## Summary

The library advertises `#[no_std]` support (via `default-features = false`), but does not document the critical behavior that `Soul::drop` and `Soul::sever` **block the current thread** when Liches are still alive. In a `no_std` environment without OS-level futex support, `atomic_wait` falls back to **spin-waiting**, which consumes 100% CPU on that core. This should be prominently documented.

## Why This Is an Issue

The blocking behavior is the core safety mechanism of the library:
- `Soul::drop` calls `sever::<true>()`, which loops calling `sync::wait(count, value)` until the count reaches 0.
- `sync::wait` calls `atomic_wait::wait`, which uses platform-specific futex/WaitOnAddress.
- In `no_std` environments, `atomic_wait` uses a spin-wait fallback.

The README mentions:
> *This library supports `#[no_std]` (use `default-features = false` in your 'Cargo.toml').*

But it does not mention:
1. That `Soul::drop` blocks the thread (busy-spins in `no_std`).
2. That this can cause 100% CPU utilization in `no_std` contexts.
3. That `Soul::sever` has the same behavior.
4. The deadlock risk if the last `Lich` is held on the same thread as the `Soul`.

## Impact

- **Severity**: Medium (documentation gap)
- **Affected users**: Anyone using the library in `no_std` or embedded environments.
- **Consequence**: Users may be surprised by CPU-burning spin-waits or deadlocks.

## Proposed Fix

### 1. Add a `# no_std` section to the crate-level documentation

In `phylactery/README.md` (which is included as the crate doc via `#![doc = include_str!("../README.md")]`):

```markdown
### `no_std` Support

This library supports `#[no_std]` by disabling the `std` feature (`default-features = false`).

**Important**: In `no_std` environments, `Soul::drop` and `Soul::sever` use **spin-waiting**
(via `atomic_wait`'s no_std fallback) when Liches are still alive. This means:

- The blocking thread will consume 100% CPU until all Liches are dropped.
- If the last Lich is held on the same thread, a **deadlock** will occur.
- For embedded or real-time systems, ensure all Liches are dropped before the Soul goes out of scope.
```

### 2. Add documentation to `Soul::sever` and `Soul::drop`

The doc comments on `sever` should mention the blocking behavior:

```rust
/// # Blocking
///
/// This method blocks the current thread until all bound [`Lich`]es are dropped.
/// In `no_std` environments, this uses spin-waiting (busy-wait), which consumes
/// 100% CPU on the current core.
```

### 3. Add a "Deadlock" section to `Soul`'s type-level documentation

```rust
/// # Deadlock Risk
///
/// If the calling thread holds the last [`Lich`] when a [`Soul`] is dropped,
/// the drop will deadlock: the thread will block waiting for the Lich to be
/// dropped, but the Lich can't be dropped because the thread is blocked.
///
/// To avoid this, ensure that Liches are dropped before the Soul, or use
/// [`try_sever`](Soul::try_sever) for a non-blocking alternative.
```

## Files to Modify

- `phylactery/README.md` (both root and `phylactery/README.md`): Add `no_std` section with blocking details.
- `phylactery/src/soul.rs` (lines 20-52): Add deadlock and blocking documentation to `Soul`'s type docs.
- `phylactery/src/soul.rs` (lines 120-142): Add blocking documentation to `Soul::sever`.

## Verification

- Documentation renders correctly: `cargo doc --all-features --no-deps --open`.
- No new warnings from `cargo doc`.

## Notes

The `Soul::sever` method already has a `# Deadlock` section (lines 130-132), which is good. However, the same information should be prominently mentioned in the type-level documentation and README, and the `no_std` spin-wait behavior should be called out explicitly.

The existing deadlock docs say:
```
/// If the calling thread holds the last [`Lich`] that keeps the count
/// non-zero, calling this method will deadlock.
```

This covers the explicit `sever` case but not the implicit case where `Soul::drop` triggers the blocking behavior.
