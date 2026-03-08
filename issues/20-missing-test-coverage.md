# Issue 20: Missing Coverage for `Soul::consume` and `sever` Edge Cases

## Summary

The test suite in `phylactery/tests/binding.rs` covers many common usage
patterns, but several important edge cases and code paths have no test coverage:

1. `Soul::consume` — the method for extracting the owned value is never tested.
2. `Soul::sever` / `Soul::try_sever` — only tested via the convenience wrappers
   (`Box::pin`), not for the `Arc::pin` or `Rc::pin` smart pointer variants.
3. Concurrent redeem + sever — the race condition described in Issue 01 has no
   test.
4. `Lich` across `std::thread::scope` boundaries — no test verifies interaction
   with scoped threads.
5. `Soul` with a value that has a non-trivial `Drop` — no test verifies that
   the inner `T::drop` is called correctly after sever.

## Location

`phylactery/tests/binding.rs` — the existing test file lacks the cases below.

## Detailed Missing Coverage

### 1. `Soul::consume`

```rust
#[test]
fn can_consume_soul() {
    let value = String::from("hello");
    let soul = Soul::new(value);
    let recovered = soul.consume();
    assert_eq!(recovered, "hello");
}

#[test]
fn consume_calls_inner_drop() {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    let dropped = Arc::new(AtomicBool::new(false));
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard {
        fn drop(&mut self) { self.0.store(true, Ordering::Relaxed); }
    }
    let soul = Soul::new(Guard(dropped.clone()));
    assert!(!dropped.load(Ordering::Relaxed));
    let guard = soul.consume();
    assert!(!dropped.load(Ordering::Relaxed));
    drop(guard);
    assert!(dropped.load(Ordering::Relaxed));
}
```

### 2. Sever with `Arc::pin`

```rust
#[test]
fn can_sever_arc_pinned_soul() {
    let soul = Arc::pin(Soul::new(|| 'a'));
    let lich = soul.as_ref().bind::<dyn Fn() -> char>();
    drop(lich);
    let soul = Soul::sever(soul);  // Arc<Soul<_>>
    assert_eq!((*soul)(), 'a');
}
```

### 3. `Soul::sever` blocks until Lich is dropped on a different thread

```rust
#[test]
fn sever_blocks_until_thread_lich_drops() {
    use std::{thread, time::Duration};
    let soul = Box::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn() + Sync>();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        drop(lich);
    });
    Soul::sever(soul);  // must block until the thread drops lich
    handle.join().unwrap();
}
```

### 4. `Soul::redeem` + concurrent sever (regression test for Issue 01)

```rust
#[test]
fn redeem_wakes_sever_thread() {
    use std::{sync::Arc, thread, time::Duration};
    let soul = Arc::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn()>();
    let soul_for_thread = unsafe { Pin::new_unchecked(Arc::clone(&soul)) };
    let handle = thread::spawn(move || {
        Soul::sever(soul_for_thread);
    });
    thread::sleep(Duration::from_millis(10)); // let the thread park in sever
    soul.redeem(lich).unwrap();  // should wake the thread
    handle.join().expect("sever thread should complete");
}
```

### 5. Inner `T::drop` called after Soul drop

```rust
#[test]
fn soul_drop_calls_inner_drop() {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    let dropped = Arc::new(AtomicBool::new(false));
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard { fn drop(&mut self) { self.0.store(true, Ordering::Relaxed); } }
    impl Fn<()> for Guard { … }  // simplified
    let soul = Box::pin(Soul::new(Guard(dropped.clone())));
    assert!(!dropped.load(Ordering::Relaxed));
    drop(soul);
    assert!(dropped.load(Ordering::Relaxed), "Soul::drop must drop the inner T");
}
```

### 6. `Lich::bindings` and `Soul::bindings` accuracy

```rust
#[test]
fn bindings_after_sever_returns_zero() {
    let soul = Box::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn()>();
    assert_eq!(lich.bindings(), 1);
    drop(lich);
    // Now count is 0, then sever sets it to u32::MAX
    let soul = Soul::sever(soul);
    // After sever, the soul is unpinned; bindings() would report 0
    // (u32::MAX sentinel maps to 0)
    // (This tests the wrapping arithmetic in bindings())
}
```

## Plan to Fix

Add the missing tests to `phylactery/tests/binding.rs` (or a new
`tests/edge_cases.rs` file).  Each test should:
1. Test one specific edge case or code path.
2. Be named descriptively so that CI failures are easy to diagnose.
3. Not depend on timing (avoid `thread::sleep` where possible; use channels or
   atomic flags for synchronisation).

After adding tests, run `cargo test --all-features` and
`cargo +nightly miri test --all-features` to confirm they pass.
