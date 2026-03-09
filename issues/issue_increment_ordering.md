# Issue: `increment` Uses Overly Strong Memory Ordering (`Acquire` Instead of `Relaxed`)

## Summary

The `increment` function in `phylactery/src/lich.rs` uses `Ordering::Acquire` for the success case of its `fetch_update` CAS loop. This is unnecessary because the caller already has access to the data through an existing reference, and no synchronization with prior writers is needed. Using `Relaxed` for the success ordering would be correct and potentially more efficient on weakly-ordered architectures (e.g., ARM).

## Why This Is an Issue

The `increment` function is called from two places:
1. `Soul::bind()` (line 95 of `soul.rs`) — the caller holds `Pin<&Self>`, which provides shared access to the Soul's data.
2. `Lich::clone()` (line 98 of `lich.rs`) — the caller holds `&Lich`, which already provides access to the data via `Deref`.

In both cases, the caller **already has a valid reference** to the data. The `Acquire` ordering in `increment` synchronizes with... nothing useful:
- There are no prior `Release` stores that the incrementing thread needs to see.
- The data is immutable (only `&T` access is provided), so no writes need to be synchronized.
- The count itself is just a counter — the caller doesn't use the count value to gate data access.

Here's the current code:

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

The `Acquire` on success is conservative (it's never wrong to use stronger ordering), but it adds unnecessary synchronization overhead.

## Impact

- **Severity**: Low (performance)
- **No correctness issue** — stronger ordering is always safe, just potentially slower.
- **Affected architectures**: On x86/x86-64, Acquire and Relaxed have the same cost for CAS operations. On ARM, RISC-V, and other weakly-ordered architectures, Acquire requires additional memory barrier instructions.

## Proposed Fix

Change the success ordering from `Acquire` to `Relaxed`:

```rust
pub(crate) fn increment(count: &AtomicU32) -> u32 {
    let result = count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
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

## Files to Modify

- `phylactery/src/lich.rs` (line 182): Change `Ordering::Acquire` to `Ordering::Relaxed`.

## Verification

- All existing tests must pass.
- Loom tests (`RUSTFLAGS="--cfg loom" cargo test --all-features --test loom`) must pass.
- Run under Miri to verify no data races.

## Notes

The `decrement` function correctly uses `Release` ordering (line 200), which synchronizes with the `Acquire` in `sever`'s CAS. This is critical for correctness and should NOT be changed. Only `increment`'s success ordering is unnecessarily strong.

A comment should be added explaining why `Relaxed` is sufficient:

```rust
// Relaxed is sufficient here because the caller already has a reference to the
// Soul's data (via Pin<&Self> in bind() or via &Lich in clone()), so no
// synchronization with prior writes is needed.
```
