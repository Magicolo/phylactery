# Issue 13: `sever()` Contains Dead Match Arm `Ok(u32::MAX)`

## Summary

The `sever` free function in `soul.rs` contains a match arm `Ok(u32::MAX)` that
can never be reached.  The arm is dead code: `compare_exchange(0, u32::MAX, …)`
returns `Ok(old_value)` only when the old value equals the first argument `0`,
so `Ok(u32::MAX)` would require `old_value` to be simultaneously `0` and
`u32::MAX` — which is impossible.  The dead arm reduces code clarity and may
confuse future maintainers.

## Location

`phylactery/src/soul.rs`, `sever`, lines 190-198.

```rust
fn sever<const FORCE: bool>(count: &AtomicU32) -> bool {
    loop {
        match count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
            Ok(0 | u32::MAX) | Err(u32::MAX) => break true,   // Ok(u32::MAX) is dead
            Ok(value) | Err(value) if FORCE => atomic_wait::wait(count, value),
            Ok(_) | Err(_) => break false,
        }
    }
}
```

## Detailed Explanation

`AtomicU32::compare_exchange(expected, new, …)`:
- Returns `Ok(old)` if `old == expected` (i.e., the exchange succeeded), where
  `old` is the value that was in the atomic before the exchange.
- Returns `Err(current)` if `old != expected`, where `current` is the value
  currently in the atomic.

In `sever`, `expected = 0`.  Therefore:
- `Ok(value)` means the old value was `0` (exchange succeeded), so `value`
  can only be `0`.  `Ok(u32::MAX)` is impossible.
- `Err(value)` means the old value was **not** `0`.  `Err(u32::MAX)` means the
  old value was `u32::MAX` (i.e., `sever` was already called by another thread
  or a previous invocation).

The first match arm `Ok(0 | u32::MAX)` is therefore equivalent to `Ok(0)`, and
the `| u32::MAX` part is dead code.

### Additional problem: `Ok(value)` arm now matches `Ok(0)` unintentionally

Because the first arm claims `Ok(0 | u32::MAX)` and the second arm is
`Ok(value) | Err(value) if FORCE`, any `Ok(0)` that falls through to the second
arm would be treated as a live binding — but `Ok(0)` means the CAS succeeded,
which is the success case and is already handled by the first arm.  The arms
are correct in practice because `Ok(0)` is caught first, but the redundant
`u32::MAX` in the first arm makes reasoning about the arms harder.

## Plan to Fix

Simplify the first arm to remove the dead pattern:

```rust
fn sever<const FORCE: bool>(count: &AtomicU32) -> bool {
    loop {
        match count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
            // CAS succeeded (old value was 0), or count was already u32::MAX
            // (sever was already called by another path).
            Ok(0) | Err(u32::MAX) => break true,
            // CAS failed: count is non-zero and non-sentinel; Liches still alive.
            Ok(value) | Err(value) if FORCE => atomic_wait::wait(count, value),
            Ok(_) | Err(_) => break false,
        }
    }
}
```

Add an inline comment documenting why `Ok(u32::MAX)` cannot occur, so future
readers understand the invariant without having to re-derive it.

Optionally, add a `debug_assert_ne!(value, u32::MAX)` in the `FORCE` wait arm
to catch any future regression where the sentinel leaks into the live-count
range:

```rust
Ok(value) | Err(value) if FORCE => {
    debug_assert_ne!(value, u32::MAX, "sentinel in live-count range");
    atomic_wait::wait(count, value);
}
```

Run `cargo test` and `cargo miri test --all-features` after the change to
confirm no regressions.
