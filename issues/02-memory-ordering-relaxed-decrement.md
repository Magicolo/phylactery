# Issue 02: Memory-Ordering Bug – `decrement` Uses `Relaxed`, but `sever`'s CAS Uses `Acquire`

## Summary

`Lich::drop` decrements the shared counter with `Ordering::Relaxed`.  The
`sever` function in `soul.rs` uses `Ordering::Acquire` on the success path of
its `compare_exchange`.  On weakly ordered architectures (e.g. ARM, POWER) this
means there is **no happens-before relationship** between a Lich drop and the
subsequent successful CAS in `sever`.  Additionally, the `wake_one` call in
`Lich::drop` happens *after* the Relaxed decrement, which creates a theoretical
window where the Soul can be fully deallocated before `wake_one` accesses the
(now-freed) counter memory.

## Location

- `phylactery/src/lich.rs` – `decrement`, lines 114-119
- `phylactery/src/lich.rs` – `Lich::drop`, lines 90-97
- `phylactery/src/soul.rs` – `sever`, lines 190-198

## Detailed Explanation

### Memory model requirements (Rust / C++ memory model)

The Rust memory model (based on C++20) guarantees a happens-before edge between
a *Release* store and an *Acquire* load **on the same atomic** only when the
Acquire load observes the value written by the Release store.

A *Relaxed* store does NOT form a synchronises-with (and therefore
happens-before) relationship with any Acquire load, even if that load observes
the stored value.  On x86 (TSO) the processor model happens to be stronger and
this distinction is invisible at run time, but on ARM, POWER, and RISC-V the
processor can legally reorder or delay visibility of Relaxed stores relative to
Acquire loads.

### Current code

```rust
// lich.rs – decrements with Relaxed
pub(crate) fn decrement(count: &AtomicU32) -> u32 {
    match count.fetch_sub(1, Ordering::Relaxed) {
        0 | u32::MAX => unreachable!(),
        value => value - 1,
    }
}
```

```rust
// soul.rs – sever uses Acquire on success, Relaxed on failure
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

### Problem 1: No synchronisation between Lich drops and Soul sever

For the Soul to safely drop its `value: T` after `sever` returns, the drop
implementation of `T` must be sequenced after any access by any thread that
previously held a `Lich`.  In other words, *all operations performed by Lich
holders must happen-before the Soul destroys its value*.

With Relaxed decrements, there is no Rust/C++ memory-model guarantee that the
operations performed by Lich-holding threads are visible to the thread running
`sever` after the CAS succeeds.  On a weakly ordered machine:

1. Thread A (Lich holder): reads through `lich.value` (arbitrary data
   accesses), then calls `Lich::drop` → Relaxed `fetch_sub`.
2. Thread B (Soul owner): CAS succeeds with Acquire.  But because there is no
   Release in Thread A, the Acquire in Thread B does **not** establish
   happens-before with Thread A's memory accesses.
3. Thread B drops `soul.value: T`.  The drop sees a potentially incoherent view
   of any shared mutable state.

If `T` itself contains only immutable data (`&`-only access through `Lich`),
this may not manifest as a practical bug, since the value is never mutated
through a `Lich` (only `&T` is handed out).  However, any interior mutability
inside `T` (e.g., `Mutex<U>`, `AtomicXxx`) could in theory observe races when
the Soul's `Drop` interacts with the final accesses by Lich-holding threads.

### Problem 2: `wake_one` on potentially freed Soul memory

After the last Lich decrements the counter to zero with `Relaxed` ordering,
`Lich::drop` calls `wake_one` using a reference into `Soul.count`:

```rust
impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        let count = self.count_ref(); // reference to Soul.count
        if decrement(self.count_ref()) == 0 {
            atomic_wait::wake_one(count); // accessed AFTER decrement
        }
    }
}
```

The race window on a weakly ordered machine:

1. Last Lich: Relaxed `fetch_sub` → count = 0.
2. Soul (sever loop): CAS observes count = 0 (Acquire), sets count = u32::MAX.
   Immediately continues: `sever` returns, `Drop::drop` returns, the Soul's
   fields are destroyed, and the allocator **frees the Soul's memory** (which
   includes the `count` field).
3. Last Lich: calls `wake_one(count)` — `count` now points to freed memory.

This is **undefined behaviour** per the Rust / C++ memory model: a reference to
freed memory is accessed after the memory has been deallocated.

In practice this race is extremely narrow and very unlikely on x86 (TSO), but
it is legal on ARM/POWER and would be caught by tools like Miri or LSAN under
some schedules.

### Why the current tests pass

- The test suite runs on x86 where processor ordering effectively prevents this
  from surfacing.
- Miri uses a sequentially consistent model in many situations and may not
  explore all weak-memory interleavings without explicit configuration.

## Plan to Fix

### Step 1: Change `decrement` to use `Release`

```rust
pub(crate) fn decrement(count: &AtomicU32) -> u32 {
    match count.fetch_sub(1, Ordering::Release) {
        0 | u32::MAX => unreachable!(),
        value => value - 1,
    }
}
```

With `Release` on the fetch_sub and `Acquire` on the successful CAS in `sever`,
a proper synchronises-with edge is established: after the CAS, the Soul thread
is guaranteed to observe all writes performed by the last Lich-holding thread
before its Release decrement.

### Step 2: Prevent wake_one from using a dangling reference

The fundamental issue is that `wake_one(count)` is called after the Relaxed
(or even Release) decrement, but the Soul could race to free the memory
containing `count`.

One solution is to ensure the Soul does not free its memory until after all
`Lich::drop` destructors have called `wake_one`.  A clean approach that avoids
introducing a second atomic is to move `wake_one` to *before* the final `Drop`
invocation by using a two-phase protocol: the last Lich sets a "wake needed"
flag atomically, and the Soul polls this before proceeding.

A simpler mitigation: give `count` its own heap allocation (via `Arc<AtomicU32>`)
so both the `Lich` and the `Soul` share ownership of the counter.  Neither party
frees the counter until both release their `Arc` handle.  This eliminates the
dangling reference entirely.

**Recommended approach** (least API surface change):
- Change `decrement` to `Release`.
- Add a `SeqCst` fence (or a `Release` fence in `Lich::drop`) after `wake_one`
  to ensure the wake call is sequenced before the Lich fully drops.
- Consider separating the `count` field into an `Arc<AtomicU32>` shared between
  `Soul` and `Lich` to guarantee the counter outlives both.

### Tests to add

- A targeted multi-threaded test using `loom` (or Miri with
  `-Zmiri-disable-stacked-borrows` + weak memory emulation) that verifies the
  correct ordering.
- An integration test where many Liches drop concurrently from different threads
  while the Soul simultaneously calls `sever`, checking for no data races.
