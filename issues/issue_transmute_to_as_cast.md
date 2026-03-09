# Issue: Unnecessary `transmute` in Shroud Implementations — Use `as` Casts Instead

## Summary

The `Shroud` trait implementations (both in the built-in macros and the `#[shroud]` proc-macro) use `core::mem::transmute` to convert between fat pointer types where simpler `as` pointer casts would suffice. Replacing `transmute` with `as` casts would reduce the unsafe surface area, make the code more readable, and eliminate a class of potential bugs.

## Why This Is an Issue

Per the Rust Nomicon, `transmute` is the most dangerous unsafe operation because it bypasses all type checking. The general guidance is: **avoid `transmute` whenever a safer alternative exists**.

In the current code, the pattern is:

```rust
// phylactery/src/shroud.rs, line 110-119 (concrete-to-dyn case)
fn shroud(from: ::core::ptr::NonNull<TConcrete>) -> ::core::ptr::NonNull<Self> {
    unsafe {
        ::core::ptr::NonNull::new_unchecked(::core::mem::transmute::<
            *mut (dyn Fn(...) -> R + Traits),
            *mut Self
        >(from.as_ptr() as _))
    }
}
```

This performs two steps:
1. `from.as_ptr() as _` — unsizing coercion from `*mut TConcrete` to `*mut dyn Trait` (via `as` cast)
2. `transmute::<*mut dyn Trait, *mut Self>` — identity or lifetime change

For the **concrete-to-dyn case** (where `Self` is exactly `dyn Trait`), the transmute is a **no-op** — the types are identical. The `as _` cast already produces the correct `*mut dyn Trait`.

For the **dyn-to-dyn case** (lifetime extension, in the proc-macro), the transmute changes the lifetime of the fat pointer:
```rust
// phylactery_macro/src/lib.rs, line 70-74
transmute::<
    *mut (dyn Trait + '__life_in__),
    *mut (dyn Trait + '__life_out__)
>(from.as_ptr() as _)
```

In Rust 1.75+ (the crate's MSRV), raw pointer casts via `as` can perform both unsizing coercions and lifetime changes for `*mut dyn Trait` pointers. The entire `transmute` can be replaced with a single `as` cast.

## Impact

- **Severity**: Low (code quality / safety hygiene)
- **No runtime behavior change** — the generated code is identical at the machine level.
- **Benefit**: Reduced unsafe surface area, clearer intent, and elimination of a footgun.

## Proposed Fix

### For the built-in implementations in `phylactery/src/shroud.rs`:

Replace the transmute pattern with a direct `as` cast:

```rust
// Before (concrete-to-dyn)
fn shroud(from: ::core::ptr::NonNull<TConcrete>) -> ::core::ptr::NonNull<Self> {
    unsafe {
        ::core::ptr::NonNull::new_unchecked(::core::mem::transmute::<
            *mut (dyn $function($($parameter),*) -> $return $(+ $trait)*),
            *mut Self
        >(from.as_ptr() as _))
    }
}

// After
fn shroud(from: ::core::ptr::NonNull<TConcrete>) -> ::core::ptr::NonNull<Self> {
    unsafe {
        ::core::ptr::NonNull::new_unchecked(from.as_ptr() as *mut Self)
    }
}
```

### For the proc-macro generated code in `phylactery_macro/src/lib.rs`:

```rust
// Before (dyn-to-dyn lifetime extension)
fn shroud(from: NonNull<dyn Trait + '__life_in__>) -> NonNull<Self> {
    unsafe {
        NonNull::new_unchecked(transmute::<
            *mut (dyn Trait + '__life_in__),
            *mut Self
        >(from.as_ptr() as _))
    }
}

// After
fn shroud(from: NonNull<dyn Trait + '__life_in__>) -> NonNull<Self> {
    unsafe {
        NonNull::new_unchecked(from.as_ptr() as *mut Self)
    }
}
```

### For `shroud_ty!` macro in `phylactery/src/shroud.rs`:

Apply the same change to all three arms of the macro (lines 27-65).

## Files to Modify

- `phylactery/src/shroud.rs`: All `shroud_ty!` and `shroud_fn!` macro arms (lines 27-65 and 93-121).
- `phylactery_macro/src/lib.rs`: Both the dynamic and concrete impl generation (lines 64-94).

## Verification

- All existing tests must pass.
- Run under Miri (both Stacked Borrows and Tree Borrows) to verify no provenance issues.
- The `as` casts have been verified to work on MSRV 1.75+ per previous investigation.

## Notes

This change removes the `use` of `core::mem::transmute` from the generated code, which is a significant safety improvement. The `unsafe` block for `NonNull::new_unchecked` still remains (since the pointer is known to be non-null from the `NonNull` input).
