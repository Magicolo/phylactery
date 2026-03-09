# Issue: `combinations()` Overflow Risk for Large Inputs

## Summary

The `combinations()` function in `phylactery_macro/src/shroud.rs` computes `1usize << items.len()` without guarding against overflow. If `items.len()` >= `usize::BITS` (64 on 64-bit systems, 32 on 32-bit systems), the shift will overflow, causing a **panic in debug mode** or **silent wrap-around to 0 in release mode**.

## Why This Is an Issue

Here's the current code:

```rust
// phylactery_macro/src/shroud.rs, line 121-139
fn combinations<T>(items: &[T]) -> Vec<Vec<&T>> {
    let count = 1usize << items.len();  // ← OVERFLOW if items.len() >= usize::BITS
    let mut groups = Vec::with_capacity(count);
    for mask in 0..count {
        let group: Vec<&T> = items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if mask & (1 << i) != 0 {
                    Some(item)
                } else {
                    None
                }
            })
            .collect();
        groups.push(group);
    }
    groups
}
```

The function is called from `Shroud::paths()` when the user specifies `..` (combine mode) in the `#[shroud]` attribute:

```rust
#[shroud(Send, Sync, Unpin, ..)]
pub trait Combine {}
```

With 3 marker traits, this generates `2^3 = 8` combinations — fine. But if a user accidentally (or intentionally) specifies a very large number of traits, the exponential growth would be problematic:
- 20 traits → 2^20 = 1,048,576 combinations (already excessive)
- 64 traits → overflow/panic

## Impact

- **Severity**: Low (macro-time panic, not runtime UB)
- **Likelihood**: Very low — users rarely specify more than a handful of marker traits.
- **Consequence**: The proc-macro would panic during compilation, giving an unhelpful error message.

## Proposed Fix

Add a guard at the top of `combinations()` (or in the calling code) that limits the number of items and provides a clear error message:

```rust
fn combinations<T>(items: &[T]) -> Vec<Vec<&T>> {
    assert!(
        items.len() < usize::BITS as usize,
        "combinations: too many items ({}) — maximum is {}",
        items.len(),
        usize::BITS - 1
    );
    let count = 1usize << items.len();
    // ... rest unchanged
}
```

Alternatively, a more practical limit could be imposed (e.g., 16 or 20 traits) with a helpful compile error:

```rust
const MAX_COMBINE_TRAITS: usize = 16;

fn combinations<T>(items: &[T]) -> Vec<Vec<&T>> {
    if items.len() > MAX_COMBINE_TRAITS {
        panic!(
            "#[shroud(..)] combine mode supports at most {} traits, got {}",
            MAX_COMBINE_TRAITS, items.len()
        );
    }
    // ...
}
```

## Files to Modify

- `phylactery_macro/src/shroud.rs` (line 121-122): Add overflow guard.

## Verification

- Existing tests must pass.
- Add a test verifying the guard fires for large inputs.

## Notes

Since this is a proc-macro (compile-time code), the overflow would cause a compile-time panic, not a runtime issue. However, the error message from an overflow panic (`attempt to shift left with overflow`) is much less helpful than a clear assertion message.
