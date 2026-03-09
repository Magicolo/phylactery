# Issue: `Soul::Drop` Can Block Indefinitely When Called During Panic Unwinding

## Summary

When a `Soul` is dropped during panic unwinding (e.g., on a thread that panics while the Soul is on the stack), `<Soul as Drop>::drop` calls `sever::<true>`, which blocks the current thread until all `Lich`es are dropped. If another thread also panics while holding a `Lich`, the `Soul`'s drop will block indefinitely (dead thread holding a Lich) and the unwinding will never complete. This interacts poorly with the standard `abort` behavior for double-panics.

## Why This Is an Issue

Consider this scenario:

```rust
use std::thread;
use core::pin::pin;

let soul = Box::pin(Soul::new(|| {}));
let lich1 = soul.as_ref().bind::<dyn Fn() + Sync>();
let lich2 = soul.as_ref().bind::<dyn Fn() + Sync>();

// Thread 1 gets lich1
let h1 = thread::spawn(move || {
    let _l = lich1;
    panic!("thread 1 panic");
    // lich1 is dropped during unwinding ← this works fine
});

// Thread 2 gets lich2
let h2 = thread::spawn(move || {
    let _l = lich2;
    // If this thread gets stuck (e.g., deadlock, infinite loop),
    // lich2 is never dropped
    loop { std::thread::park(); }
});

// Main thread: Soul drops here, blocks forever waiting for lich2
drop(soul);
```

In this scenario, `Soul::drop` blocks the main thread indefinitely. This is the **documented behavior** (the library warns about deadlocks), but there are some subtle implications:

1. **During panic unwinding**: If the main thread panics while the Soul is on the stack, `Soul::drop` runs during unwinding. If it blocks, the unwinding stalls. If another panic occurs during this stall, the process aborts.

2. **Thread pool scenarios**: In a thread pool, if a worker thread panics and the Soul is on that thread, the Soul's drop could block the worker, preventing it from processing other tasks.

3. **No timeout mechanism**: There's no way to configure a timeout for the blocking wait. `try_sever` is non-blocking but doesn't help during `Drop`.

## Impact

- **Severity**: Medium (design limitation / documentation)
- **No soundness issue** — the blocking is the safety mechanism that prevents use-after-free.
- **Affected users**: Anyone using `Soul` in panic-prone code paths.

## Proposed Fix

This is primarily a documentation issue. The library should clearly document:

### 1. Add to `Soul`'s type-level documentation:

```rust
/// # Panics and Unwinding
///
/// The [`Drop`] implementation of [`Soul`] blocks the current thread until
/// all bound [`Lich`]es are dropped. This means that if a [`Soul`] is
/// dropped during panic unwinding, the unwinding will stall until all
/// [`Lich`]es are dropped on other threads.
///
/// If a thread holding a [`Lich`] is itself stuck (deadlocked, parked, or
/// in an infinite loop), the [`Soul`]'s drop will block indefinitely. In
/// a panic context, this can prevent the process from completing its abort
/// sequence.
///
/// **Recommendation**: In code that might panic, prefer using
/// [`try_sever`](Soul::try_sever) to explicitly manage the Soul's lifecycle
/// before the panic-prone section, or ensure that all [`Lich`]es are
/// short-lived and guaranteed to be dropped promptly.
```

### 2. Consider adding `Soul::sever_or_abort` or a timeout-based sever:

A more advanced fix would be to add a method like:

```rust
/// Attempts to sever with a timeout. If the timeout expires, returns
/// an error without severing.
pub fn sever_timeout<S: Deref<Target = Self>>(
    this: Pin<S>,
    timeout: Duration,
) -> Result<S, Pin<S>> { ... }
```

However, this would require `std` (for timing) and is out of scope for a minimal fix.

## Files to Modify

- `phylactery/src/soul.rs`: Add documentation to `Soul`'s type-level docs and `Drop` implementation.
- `phylactery/README.md`: Add a note about panic behavior.

## Verification

- Documentation renders correctly.
- No code changes needed unless implementing timeout-based sever.

## Notes

The existing tests `unwinds_on_same_thread` and `unwinds_on_different_threads` (both `#[should_panic]`) demonstrate that the library handles panics correctly when the Lich is dropped during unwinding on the same thread. The issue is specifically about **cross-thread** scenarios where the thread holding the Lich cannot drop it.

The `#[should_panic]` test `unwinds_on_different_threads` actually covers a related case, but it relies on `sleep` timing and doesn't test the "stuck thread" scenario.
