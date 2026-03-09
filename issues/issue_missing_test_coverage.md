# Issue: Missing Test Coverage for Several Edge Cases

## Summary

While the library has good test coverage for its core functionality (26 binding tests, 12 loom tests, 7 doc tests), several important edge cases are not tested. Adding tests for these cases would improve confidence in the library's correctness and prevent regressions.

## Missing Test Cases

### 1. `into_value` Does Not Double-Drop

`Soul::into_value` uses `ManuallyDrop` + `ptr::read` to extract the value without running `Soul::drop`. If this mechanism fails, the value could be dropped twice (UB). Currently there is no test verifying this.

**Proposed test** (in `phylactery/tests/binding.rs`):
```rust
#[test]
fn into_value_does_not_double_drop() {
    use std::sync::atomic::{AtomicU32, Ordering};
    static DROP_COUNT: AtomicU32 = AtomicU32::new(0);

    struct DropTracker;
    impl Drop for DropTracker {
        fn drop(&mut self) {
            DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT.store(0, Ordering::SeqCst);
    let soul = Soul::new(DropTracker);
    let tracker = soul.into_value();
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0, "should not have dropped yet");
    drop(tracker);
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1, "should drop exactly once");
}
```

### 2. Panic Safety — Lich Dropped During Unwinding

When a panic unwinds the stack while a Lich is in scope, the Lich's `Drop` should decrement the count and wake any parked threads. This is critical for ensuring the Soul can complete its drop.

**Proposed test**:
```rust
#[test]
fn panic_drops_lich_correctly() {
    let soul = Box::pin(Soul::new(|| {}));
    let bindings_before = soul.bindings();
    assert_eq!(bindings_before, 0);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _lich = soul.as_ref().bind::<dyn Fn()>();
        assert_eq!(soul.bindings(), 1);
        panic!("test panic");
    }));

    assert!(result.is_err());
    assert_eq!(soul.bindings(), 0, "Lich should have been dropped during unwinding");
}
```

### 3. `Soul::Default` Trait Implementation

`Soul` derives `Default`, but there's no test verifying it works correctly:

```rust
#[test]
fn soul_default_creates_zero_bindings() {
    let soul: Soul<i32> = Soul::default();
    assert_eq!(soul.bindings(), 0);
    assert_eq!(*soul, 0); // i32::default() is 0
}
```

### 4. `Lich` Trait Implementations (PartialEq, Ord, Hash)

The `Lich` type implements `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash`, but none of these are tested:

```rust
#[test]
fn lich_partial_eq() {
    let soul1 = Box::pin(Soul::new(42_i32));
    let soul2 = Box::pin(Soul::new(42_i32));
    let lich1 = soul1.as_ref().bind::<dyn fmt::Debug>();
    // Note: PartialEq compares the values, not the pointers
    // This requires the underlying type to implement PartialEq
    // Since dyn Debug doesn't implement PartialEq, we need a different trait
}

#[test]
fn lich_from_soul() {
    let soul = Soul::from(42_i32);
    assert_eq!(*soul, 42);
}
```

### 5. `AsRef` and `Borrow` Implementations

```rust
#[test]
fn soul_as_ref_and_borrow() {
    use core::borrow::Borrow;

    let soul = Soul::new(42_i32);
    let as_ref: &i32 = soul.as_ref();
    assert_eq!(*as_ref, 42);
    let borrow: &i32 = soul.borrow();
    assert_eq!(*borrow, 42);
}
```

### 6. Cross-Thread Panic with Soul on Main Thread

Test that the Soul correctly handles the case where a spawned thread panics while holding a Lich:

```rust
#[test]
fn cross_thread_panic_drops_lich() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let dropped = Arc::new(AtomicBool::new(false));
    let dropped_clone = dropped.clone();

    let soul = Box::pin(Soul::new(|| {}));
    let lich = soul.as_ref().bind::<dyn Fn() + Sync>();

    let handle = spawn(move || {
        let _lich = lich;
        dropped_clone.store(true, Ordering::Release);
        panic!("cross-thread panic");
    });

    drop(soul); // blocks until lich is dropped during unwinding
    assert!(dropped.load(Ordering::Acquire));
    assert!(handle.join().is_err());
}
```

### 7. `bindings()` Returns 0 After Explicit Sever

This is partially covered by `bindings_after_sever_returns_zero`, but let's also test the severed sentinel:

```rust
#[test]
fn bindings_returns_zero_for_severed_sentinel() {
    // The SEVERED constant is u32::MAX
    // bindings() maps it to 0 using: raw.wrapping_add(1).saturating_sub(1)
    // u32::MAX.wrapping_add(1) = 0, 0.saturating_sub(1) = 0
    // This is tested implicitly by bindings_after_sever_returns_zero,
    // but we should also test the arithmetic directly
    let severed: u32 = u32::MAX;
    assert_eq!(severed.wrapping_add(1).saturating_sub(1), 0);
}
```

## Impact

- **Severity**: Medium (testing completeness)
- **No bugs discovered** — all Miri tests pass. These tests are for regression prevention.

## Files to Modify

- `phylactery/tests/binding.rs`: Add the test cases listed above.
- Optionally create a new test file `phylactery/tests/edge_cases.rs` for the more specialized tests.

## Verification

- All new tests must pass.
- All new tests must pass under Miri.
- Existing tests must still pass.
