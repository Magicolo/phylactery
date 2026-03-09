# Issue: Missing `#[deny(unsafe_op_in_unsafe_fn)]` Lint

## Summary

The `phylactery` crate does not enable the `unsafe_op_in_unsafe_fn` lint. This means that within `unsafe fn` bodies, individual unsafe operations do not require their own `unsafe` blocks. Enabling this lint improves safety reasoning by forcing each unsafe operation to be explicitly marked, making it clearer which specific operations are unsafe and why.

## Why This Is an Issue

The Rust Nomicon and the Rust API Guidelines recommend that unsafe functions should still use `unsafe` blocks around their individual unsafe operations:

> "The point of unsafe-in-unsafe-fn is that when you see unsafe { ... } inside an unsafe fn, you know the author has considered the safety requirements of that specific operation."

Currently, the crate has several `unsafe fn` that perform multiple unsafe operations:

```rust
// phylactery/src/soul.rs, line 164-168
unsafe fn unpin<S: Deref<Target = Self>>(this: Pin<S>) -> S {
    debug_assert_eq!(this.bindings(), 0);
    // Safety: no `Lich`es are bound, the `Soul` can be unpinned.
    unsafe { Pin::into_inner_unchecked(this) }
}
```

This function already uses an explicit `unsafe` block, which is good. But without the lint, future contributors might add unsafe operations without wrapping them, degrading the safety documentation.

In `lich.rs`:

```rust
// phylactery/src/lich.rs, line 74-81
unsafe fn redeem_unchecked(&self) -> usize {
    let count = self.count_ref();
    let remain = decrement(count);
    if remain == 0 {
        sync::wake_all(count);
    }
    remain as _
}
```

This function calls `count_ref()` which internally does `unsafe { self.count.as_ref() }`. While `redeem_unchecked` is `unsafe fn`, the call to `count_ref()` is safe (it's a safe method that contains its own unsafe block). So the function body contains no directly unsafe operations. Still, having the lint enabled provides a consistent policy.

## Impact

- **Severity**: Low (code quality / safety hygiene)
- **No runtime behavior change**.
- **Benefit**: Forces explicit `unsafe` blocks around individual unsafe operations within `unsafe fn`, improving documentation of safety invariants.

## Proposed Fix

Add the lint to the crate root in `phylactery/src/lib.rs`:

```rust
#![deny(unsafe_op_in_unsafe_fn)]
```

Then verify that all existing `unsafe fn` bodies either:
1. Already use explicit `unsafe` blocks (most do), or
2. Need to be wrapped in `unsafe` blocks with safety comments.

### Audit of current `unsafe fn` bodies:

1. **`Soul::unpin`** (`soul.rs:164`): Already uses `unsafe { Pin::into_inner_unchecked(this) }` ✓
2. **`Lich::redeem_unchecked`** (`lich.rs:74`): No directly unsafe operations in body ✓
3. **`Lich::count_ref`** (`lich.rs:83`): Not `unsafe fn`, uses internal `unsafe` block ✓
4. **`Lich::data_ref`** (`lich.rs:89`): Not `unsafe fn`, uses internal `unsafe` block ✓

The existing code is already well-structured. Enabling the lint would primarily serve as a guard against future regressions.

## Files to Modify

- `phylactery/src/lib.rs` (line 2): Add `#![deny(unsafe_op_in_unsafe_fn)]`.

## Verification

- `cargo build --all-features` must succeed.
- `cargo clippy --all-features` must pass.
- All tests must pass.

## Notes

The `phylactery_macro` crate already has `#![forbid(unsafe_code)]` (line 1 of `phylactery_macro/src/lib.rs`), which is even stronger — it forbids all unsafe code in the macro crate. This is correct since the macro crate only generates code, it doesn't execute unsafe operations.
