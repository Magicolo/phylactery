# Issue 01: `Soul::redeem` Does Not Wake a Sleeping `sever` Thread (Deadlock)

## Summary

`Soul::redeem` decrements the live-Lich counter but never calls
`atomic_wait::wake_one`, so a thread that is blocked inside `sever` (or inside
`<Soul as Drop>::drop`) waiting for the counter to reach zero will never wake
up if the last `Lich` is disposed of through `redeem` rather than through a
normal `drop`.

## Location

- `phylactery/src/soul.rs` – `Soul::redeem`, line 108-115
- `phylactery/src/lich.rs` – `Lich::drop`, lines 90-97; `decrement`, lines
  114-119

## Detailed Explanation

### How the wake-up protocol works

`Lich::drop` is the only place that calls `atomic_wait::wake_one`:

```rust
// lich.rs
impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        let count = self.count_ref();
        if decrement(self.count_ref()) == 0 {
            atomic_wait::wake_one(count);   // <-- the only wake-up
        }
    }
}
```

`Soul::sever` (and transitively `<Soul as Drop>::drop`) spins on a
compare-exchange and parks the thread via `atomic_wait::wait` when the count is
non-zero:

```rust
// soul.rs
fn sever<const FORCE: bool>(count: &AtomicU32) -> bool {
    loop {
        match count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
            Ok(0 | u32::MAX) | Err(u32::MAX) => break true,
            Ok(value) | Err(value) if FORCE => atomic_wait::wait(count, value), // parks
            Ok(_) | Err(_) => break false,
        }
    }
}
```

### The broken path through `redeem`

`Soul::redeem` is provided as an alternative to dropping a `Lich` directly:

```rust
// soul.rs
pub fn redeem<S: ?Sized>(&self, lich: Lich<S>) -> Result<usize, Lich<S>> {
    if self.is_bound(&lich) {
        forget(lich);          // prevents Lich::drop from running
        Ok(decrement(&self.count) as _)  // decrements count …
        // … but never calls atomic_wait::wake_one !
    } else {
        Err(lich)
    }
}
```

`forget(lich)` prevents `Lich::drop` from running, and the manual `decrement`
call does **not** call `wake_one`.  If the counter reaches zero via this path,
any thread parked in `sever` will never receive a wake-up notification.

### Confirmed reproduction

The bug has been confirmed to reproduce.  See
`phylactery/examples/issue_01_redeem_deadlock.rs` for a runnable minimal
reproduction:

```bash
cargo run --example issue_01_redeem_deadlock --features shroud
# Output with bug present:
# redeem succeeded, remaining bindings = 0
# BUG REPRODUCED: sever thread is still parked after redeem
```

A regression test has been added to `phylactery/tests/binding.rs`
(`redeem_wakes_sever_thread`).  It currently **fails** (timeout after 1 second)
and will pass once `redeem` calls `wake_one` when the count reaches zero.

```bash
cargo test redeem_wakes_sever_thread --features shroud
# FAILED (Timeout) with the bug present
```

### Minimal reproduction

```rust
use std::{sync::Arc, thread, time::Duration};
use phylactery::Soul;

fn main() {
    let soul: std::pin::Pin<Arc<Soul<fn()>>> = Arc::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn()>();

    // Clone the Arc so thread A can call sever.
    // Pin<Arc<T>>: Clone because Arc<T>: Clone always (Arc::clone never moves T).
    let soul_for_sever = soul.clone();

    // Thread A parks in sever because count == 1.
    let handle = thread::spawn(move || {
        Soul::sever(soul_for_sever); // blocks here
    });

    thread::sleep(Duration::from_millis(50)); // let thread A reach wait()

    // Thread B redeems the last Lich -- decrements count to 0 but
    // NEVER calls wake_one, so thread A stays parked forever.
    assert!(soul.redeem(lich).is_ok()); // BUG: no wake_one

    handle.join().unwrap(); // hangs forever (timeout kills it in the test)
}
```

A simpler single-threaded reproduction: Drop a `Soul` **after** all its `Lich`es
have been redeemed.  In that case `sever` will succeed immediately via the CAS
(count is already 0), so there is no deadlock in the sequential case.  The
deadlock only manifests when `sever` parks *before* `redeem` decrements the
count.

### Why `atomic_wait::wait` does not self-heal

`atomic_wait::wait(count, value)` returns early if `count != value` **at the
time the syscall is checked**.  If the decrement happens after `wait` has
already committed to sleeping (between the value check and the actual park in
the kernel), the thread stays parked indefinitely without a wake signal.
Spurious wakeups are not guaranteed to be timely or to occur at all on some
platforms.

## Why This Is a Bug

The documented contract of `Soul::redeem` is:
> *While not required, returning the Liches explicitly to the Soul ensures that
> they will all be dropped when the Soul is dropped.*

Users naturally expect `redeem` to be a safe alternative to dropping a `Lich`.
However, calling `redeem` instead of `drop` can silently create a deadlock if
`sever` is waiting concurrently, violating the liveness guarantee.

## Plan to Fix

**Option A (minimal, preferred):** Call `wake_one` in `redeem` when the
decremented count reaches zero, mirroring what `Lich::drop` does:

```rust
pub fn redeem<S: ?Sized>(&self, lich: Lich<S>) -> Result<usize, Lich<S>> {
    if self.is_bound(&lich) {
        forget(lich);
        let remaining = decrement(&self.count);
        if remaining == 0 {
            atomic_wait::wake_one(&self.count);
        }
        Ok(remaining as _)
    } else {
        Err(lich)
    }
}
```

**Option B (refactor):** Extract a shared `decrement_and_maybe_wake` helper
used by both `Lich::drop` and `Soul::redeem`.

**Tests to add:**
- A test where all `Lich`es are redeemed concurrently while the `Soul` is
  being severed / dropped, verifying no deadlock occurs.
- A test that redeems the last `Lich` after `Soul::sever` has started (using
  two threads), asserting `sever` completes within a timeout.
