# Issue 03: `combinations()` Generates Incomplete Subsets for N ≥ 4 Items

## Summary

The `combinations` function in `phylactery_macro/src/shroud.rs` is intended to
generate all non-empty subsets (power set) of its input slice so that the
`#[shroud(A, B, C, ..)]` macro expansion covers every possible combination of
marker traits.  However, the algorithm silently skips certain subsets once the
input has **four or more elements**.  For example, `{a, b, d}` is missing from
the output for `[a, b, c, d]`.

## Location

`phylactery_macro/src/shroud.rs` – `combinations`, lines 121-137, and the test
at lines 139-180.

## Detailed Explanation

### The algorithm

```rust
fn combinations<T>(mut items: &[T]) -> Vec<Vec<&T>> {
    let mut groups = Vec::with_capacity(items.len() * items.len());
    groups.push(Vec::new()); // empty set
    while let Some((head, tail)) = items.split_first() {
        groups.push(vec![head]);
        for size in 1..=tail.len() {
            for index in 0..=tail.len() - size {
                let mut group = Vec::with_capacity(size + 1);
                group.push(head);
                group.extend(&tail[index..index + size]);
                groups.push(group);
            }
        }
        items = tail;
    }
    groups
}
```

For each `head` element, the inner loops combine `head` with every *contiguous*
sub-slice of `tail` of every possible size.  Because only contiguous sub-slices
are considered, non-contiguous combinations that skip elements are omitted.

### Counting the output

| N | Expected subsets (2^N) | Produced |
|---|------------------------|----------|
| 0 | 1                      | 1        |
| 1 | 2                      | 2        |
| 2 | 4                      | 4        |
| 3 | 8                      | 8        |
| 4 | 16                     | **15**   |
| 5 | 32                     | **26**   |

For N = 3 the algorithm happens to be correct: every contiguous sub-slice of a
3-element tail covers all combinations.  For N ≥ 4 it diverges.

### Concrete missing subset for N = 4

Input: `['a', 'b', 'c', 'd']`

**Present:**  `[]`, `[a]`, `[a,b]`, `[a,c]`, `[a,d]`, `[a,b,c]`, `[a,c,d]`,
`[a,b,c,d]`, `[b]`, `[b,c]`, `[b,d]`, `[b,c,d]`, `[c]`, `[c,d]`, `[d]`

**Missing:**  `[a,b,d]`  (and more for N ≥ 5)

### The existing test encodes the wrong expectation

The test at line 160-179 *asserts the buggy output*:

```rust
assert_eq!(
    combinations(&['a', 'b', 'c', 'd']),
    vec![
        vec![],
        vec![&'a'],
        // ... 15 items total — no [&'a', &'b', &'d']
    ]
);
```

This means the test was written to match the buggy implementation rather than
the correct behaviour.

### Impact

Today the macro is only used with three marker traits (`Send`, `Sync`, `Unpin`),
for which N = 3 and the output is accidentally correct.  However:

1. If a user writes `#[shroud(A, B, C, D, ..)]` with four marker-trait
   arguments, the `Shroud` impl for `dyn Trait + A + B + D` (a non-contiguous
   combination) is silently **not generated**, leading to a confusing
   compile-time error far from the macro call site.
2. The crate documentation says `..` generates "all combinations".  The current
   implementation violates that contract for N ≥ 4.

## Plan to Fix

Replace the body of `combinations` with a correct power-set algorithm:

```rust
fn combinations<T>(items: &[T]) -> Vec<Vec<&T>> {
    let count = 1usize << items.len(); // 2^N subsets
    let mut groups = Vec::with_capacity(count);
    for mask in 0..count {
        let group: Vec<&T> = items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| if mask & (1 << i) != 0 { Some(item) } else { None })
            .collect();
        groups.push(group);
    }
    groups
}
```

**Note:** The order of generated groups will differ from the current order.
Because the order is purely an implementation detail (the generated Rust impls
are unordered), this does not affect correctness.  However, the existing tests
must be updated to reflect the new (correct) ordering—or changed to be
order-insensitive.

**Update the test** to assert all 2^N subsets are present:

```rust
#[test]
fn produces_all_combinations() {
    for n in 0..=4usize {
        let items: Vec<usize> = (0..n).collect();
        let result = combinations(&items);
        assert_eq!(result.len(), 1 << n, "wrong count for n={n}");
        // Verify every possible mask appears exactly once
        for mask in 0..(1usize << n) {
            let expected: Vec<&usize> = items
                .iter()
                .enumerate()
                .filter_map(|(i, x)| if mask & (1 << i) != 0 { Some(x) } else { None })
                .collect();
            assert!(result.contains(&expected), "missing subset {mask:b} for n={n}");
        }
    }
}
```

**Important:** The `capacity` hint `items.len() * items.len()` in the original
code should be updated to `1 << items.len()` to avoid over- or
under-allocation.  Be aware that this grows exponentially; users supplying many
traits with `..` will generate exponentially many impls.  A warning in the
documentation is appropriate.
