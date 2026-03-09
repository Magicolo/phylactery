# Issue: `decrement` Could Benefit from Debug Assertion for Double-Decrement

## Summary

The `decrement` function in `phylactery/src/lich.rs` uses `unreachable!()` to guard against double-decrement (when `fetch_sub` returns 0 or `SEVERED`). While this is correct for release builds, a `debug_assert!` with a descriptive message before the `fetch_sub` would help catch programming errors earlier during development and testing.

## Why This Is an Issue

Current code:

```rust
// phylactery/src/lich.rs, line 199-204
pub(crate) fn decrement(count: &AtomicU32) -> u32 {
    match count.fetch_sub(1, Ordering::Release) {
        0 | SEVERED => unreachable!(),
        value => value - 1,
    }
}
```

The `unreachable!()` will panic at runtime if it's hit. But by the time `fetch_sub(1)` has executed, the damage is already done — the count has already been decremented. If the old value was 0, the count is now `u32::MAX` (wrapping subtraction), which is the `SEVERED` sentinel. This means:

1. `decrement` is called when count is 0 (double-decrement bug).
2. `fetch_sub(1)` wraps around to `u32::MAX = SEVERED`.
3. The `match` hits `0 => unreachable!()` and panics.
4. But the count is now `SEVERED`, meaning any concurrent `sever` call will see `Err(SEVERED)` and think sever completed normally.
5. The Soul is then unpinned and dropped, potentially while other Liches still hold pointers.

In other words: even though the `unreachable!()` panics, the **atomic state has already been corrupted**. If the panic is caught (via `catch_unwind`), the corrupted state could lead to undefined behavior.

## Impact

- **Severity**: Low (defense-in-depth)
- **Likelihood**: Very low — double-decrement requires violating the Lich API contract.
- **Risk**: If a bug in the library or an unsafe misuse triggers double-decrement, the corrupted atomic state is worse than just a panic.

## Proposed Fix

Add a pre-condition debug assertion that loads the count before the `fetch_sub`:

```rust
pub(crate) fn decrement(count: &AtomicU32) -> u32 {
    debug_assert!(
        {
            let current = count.load(Ordering::Relaxed);
            current != 0 && current != SEVERED
        },
        "decrement called on a count that is 0 or SEVERED — double-decrement bug"
    );
    match count.fetch_sub(1, Ordering::Release) {
        0 | SEVERED => unreachable!(
            "count was 0 or SEVERED at the time of fetch_sub — double-decrement detected"
        ),
        value => value - 1,
    }
}
```

**Note**: The `debug_assert!` is a TOCTOU check (the value could change between the load and the fetch_sub), so it's not a reliable guard. But it provides a helpful diagnostic in single-threaded tests and debug builds.

Alternatively, a more robust approach would be to use `fetch_update` instead of `fetch_sub`:

```rust
pub(crate) fn decrement(count: &AtomicU32) -> u32 {
    let result = count.fetch_update(Ordering::Release, Ordering::Relaxed, |value| {
        debug_assert!(value > 0 && value != SEVERED, "double-decrement detected");
        if value > 0 && value != SEVERED {
            Some(value - 1)
        } else {
            None
        }
    });
    match result {
        Ok(old) => old - 1,
        Err(_) => unreachable!("decrement called on count 0 or SEVERED"),
    }
}
```

This approach prevents the atomic corruption because `fetch_update` only performs the subtraction if the condition is met. However, it requires a CAS loop which is slightly slower than `fetch_sub`.

## Files to Modify

- `phylactery/src/lich.rs` (lines 199-204): Add debug assertion or switch to `fetch_update`.

## Verification

- All existing tests must pass.
- Loom tests must pass.
- Miri tests must pass.

## Notes

The `fetch_update` approach provides stronger guarantees against state corruption, at the cost of slightly reduced performance (CAS loop vs. single `fetch_sub`). For a library focused on safety, this tradeoff may be worthwhile, at least in debug builds.

The current `unreachable!()` is correct under normal usage, but the defense-in-depth approach would help catch bugs in unsafe code that might call `decrement` incorrectly.
