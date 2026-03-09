# Issue: Dead `Ok(_)` Match Arms in `sever` Function

## Summary

The `sever` function in `phylactery/src/soul.rs` contains match arms with `Ok(value)` and `Ok(_)` patterns that can never match. `compare_exchange(0, SEVERED, ...)` only returns `Ok(0)` (the expected value on success), so `Ok(value)` where `value != 0` is dead code. This makes the match arms misleading and harder to reason about.

## Why This Is an Issue

Here's the current code:

```rust
// phylactery/src/soul.rs, line 221-232
fn sever<const FORCE: bool>(count: &AtomicU32) -> bool {
    loop {
        match count.compare_exchange(0, SEVERED, Ordering::Acquire, Ordering::Relaxed) {
            // `compare_exchange(0, …)` returns `Ok(old_value)` only when `old_value == 0`,
            // so only `Ok(0)` can appear here. `Err(SEVERED)` means a concurrent `sever`
            // already completed; either way, the Soul is severed.
            Ok(0) | Err(SEVERED) => break true,
            Ok(value) | Err(value) if FORCE => sync::wait(count, value),
            Ok(_) | Err(_) => break false,
        }
    }
}
```

`compare_exchange(expected=0, desired=SEVERED)` semantics:
- **Returns `Ok(old)`** only when `old == expected` (i.e., `old == 0`). So `Ok(x)` always means `x == 0`.
- **Returns `Err(actual)`** when `actual != expected`.

Therefore:
- In arm 1: `Ok(0)` matches (correct). `Err(SEVERED)` matches when another thread already severed.
- In arm 2: `Ok(value)` can only match `Ok(0)`, which was already caught by arm 1. So **only `Err(value)` can reach this arm**. The `Ok(value)` pattern is dead.
- In arm 3: Same logic — `Ok(_)` is dead. Only `Err(_)` where `_ != SEVERED` and `FORCE == false` reaches here.

The comment on line 224-226 correctly explains this, but the code structure doesn't reflect it.

## Impact

- **Severity**: Very Low (code clarity)
- **No runtime behavior change** — the dead arms are never reached.
- **Risk**: A reader might incorrectly believe `Ok(value)` where `value != 0` is possible, leading to confusion about the CAS semantics.

## Proposed Fix

Restructure the match to make the dead code explicit or remove it:

### Option A: Use `_` for the unreachable Ok arm (clearest)

```rust
fn sever<const FORCE: bool>(count: &AtomicU32) -> bool {
    loop {
        match count.compare_exchange(0, SEVERED, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) | Err(SEVERED) => break true,
            Err(value) if FORCE => sync::wait(count, value),
            Err(_) => break false,
        }
    }
}
```

This is cleaner because:
- `Ok(_)` correctly represents "CAS succeeded (value was 0)" without needing to bind the value.
- The `Err` arms clearly show the two failure cases: wait (FORCE) or give up (!FORCE).
- `Err(SEVERED)` is folded into the first arm alongside `Ok(_)`, since both mean "severed".

### Option B: Keep the existing structure but add `unreachable!()` annotations

```rust
fn sever<const FORCE: bool>(count: &AtomicU32) -> bool {
    loop {
        match count.compare_exchange(0, SEVERED, Ordering::Acquire, Ordering::Relaxed) {
            Ok(0) | Err(SEVERED) => break true,
            Err(value) if FORCE => sync::wait(count, value),
            Err(_) => break false,
            Ok(_) => unreachable!("CAS(0, _) can only succeed with old=0"),
        }
    }
}
```

## Files to Modify

- `phylactery/src/soul.rs` (lines 222-231): Restructure the match arms.

## Verification

- All existing tests must pass.
- Loom tests must pass.
- This is purely a refactor; no behavior change.
