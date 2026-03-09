# Issue: `Lich<T>` Variance Is Covariant Over `T` — Potential Lifetime Shortening Concern

## Summary

`Lich<T: ?Sized>` stores a `NonNull<T>` which is covariant over `T`. This means `Lich<dyn Trait + 'long>` can be implicitly coerced to `Lich<dyn Trait + 'short>` by the compiler. While this is generally correct (a longer-lived reference can be used where a shorter-lived one is expected), it interacts in a subtle way with the library's lifetime extension mechanism.

## Why This Is an Issue

`Lich` is defined as:
```rust
pub struct Lich<T: ?Sized> {
    pub(crate) value: NonNull<T>,
    pub(crate) count: NonNull<AtomicU32>,
}
```

`NonNull<T>` is covariant over `T` (it behaves like `*const T` for variance purposes). This means:
- `Lich<dyn Debug + 'static>` is a subtype of `Lich<dyn Debug + 'a>` for any `'a`.
- The compiler will automatically coerce the longer lifetime to the shorter one.

**Is this correct?** Yes, this is fine:
- Covariance says: if `'long: 'short`, then `Lich<dyn Trait + 'long>` can be used as `Lich<dyn Trait + 'short>`.
- This is safe: a pointer valid for a longer lifetime is also valid for a shorter one.
- The `Lich`'s `Deref` provides `&T`, so the reference's lifetime is tied to the `Lich` itself, not the trait object's lifetime parameter.

**However**, there's a subtle interaction with the `Send`/`Sync` bounds:
```rust
unsafe impl<T: ?Sized> Send for Lich<T> where for<'a> &'a T: Send {}
unsafe impl<T: ?Sized> Sync for Lich<T> where for<'a> &'a T: Sync {}
```

The `for<'a> &'a T: Send` bound is correct — it says that references to `T` of any lifetime must be `Send`. This is stronger than just `&T: Send` for a specific lifetime, and it prevents issues with lifetime covariance.

## Analysis

After careful analysis, this is **NOT a bug**. The covariance is correct:

1. **Covariance is safe for shared references**: `Lich` only provides `&T`, so covariance (shortening the lifetime) is safe. You can always use a longer-lived thing where a shorter-lived thing is expected.

2. **The HRTB `for<'a>` bound on Send/Sync is correct**: It ensures that `&'a T` is Send/Sync for ALL lifetimes, not just the trait object's lifetime parameter. This prevents a situation where `Lich<dyn Trait + 'static>` is Send but `Lich<dyn Trait + 'a>` is not.

3. **The transmute in Shroud extends lifetimes (contravariance direction)**: The Shroud transmute goes from short to long lifetime (`'__life_in__` → `'__life_out__` where `'__life_out__: '__life_in__`). This is the unsafe operation that the library's safety guarantees (Soul's drop blocking) make sound.

## Impact

- **Severity**: None (not a bug after analysis)
- This issue file documents the analysis for future reference.

## Conclusion

No fix is needed. The variance of `Lich<T>` is correct:
- **Covariance** (automatic shortening) is safe because Lich only provides shared access.
- **Contravariance** (lifetime extension) is handled by the unsafe Shroud transmute, justified by Soul's blocking drop.
- **Send/Sync bounds** use HRTB (`for<'a>`) which correctly handles all lifetimes.

This analysis should be preserved as a code comment or documentation note to help future auditors:

```rust
// Lich<T> is covariant over T, which is correct because:
// 1. Lich only provides shared (&T) access via Deref.
// 2. Shortening a trait object's lifetime is always safe.
// 3. The unsafe lifetime extension is handled by Shroud::shroud, not by variance.
```

## Files to Modify

- Optionally: `phylactery/src/lich.rs` — add a variance comment near the type definition.
