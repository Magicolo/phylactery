# Issue: Usage of `syn::__private::Span` — Private API Dependency

## Summary

The `phylactery_macro` crate imports `Span` from `syn::__private`, which is a private/internal re-export path. While this works today, private APIs in `syn` are not subject to semver guarantees and could break with any minor/patch release of `syn`. The correct approach is to import `Span` from `proc_macro2::Span` directly.

## Why This Is an Issue

In `phylactery_macro/src/shroud.rs` (line 4):

```rust
use syn::{
    __private::Span,
    // ...
};
```

The `syn::__private` module is explicitly marked as private/internal. The `syn` crate documentation states that items in this module are implementation details and may change without notice.

`Span` is re-exported from `proc_macro2::Span`. The `proc_macro2` crate is already a transitive dependency (through both `syn` and `quote`), but it's not listed as a direct dependency in `phylactery_macro/Cargo.toml`:

```toml
[dependencies]
syn = { version = "2.0.115", ... }
quote = { version = "1.0.44", ... }
# proc-macro2 is NOT listed
```

## Impact

- **Severity**: Medium (build stability)
- **Risk**: A future `syn` release could rename, remove, or restructure `__private::Span`, breaking the build.
- **Likelihood**: Low in practice (this re-export has been stable for years), but the risk is non-zero.

## Proposed Fix

1. Add `proc-macro2` as a direct dependency in `phylactery_macro/Cargo.toml`:

```toml
[dependencies]
syn = { version = "2.0.115", default-features = false, features = [
    "clone-impls",
    "proc-macro",
    "parsing",
    "full",
    "printing",
] }
quote = { version = "1.0.44", default-features = false }
proc-macro2 = { version = "1.0", default-features = false }
```

2. Update the import in `phylactery_macro/src/shroud.rs`:

```rust
// Before
use syn::{
    __private::Span,
    // ...
};

// After
use proc_macro2::Span;
use syn::{
    // ... (remove __private::Span)
};
```

## Files to Modify

- `phylactery_macro/Cargo.toml`: Add `proc-macro2` dependency.
- `phylactery_macro/src/shroud.rs` (line 4): Change import from `syn::__private::Span` to `proc_macro2::Span`.

## Verification

- `cargo build --all-features` must succeed.
- All tests must pass.
- The `proc-macro2` version should be compatible with the versions used by `syn` and `quote` (use `cargo tree` to verify).

## Notes

Since `proc_macro2` is already in the dependency tree (as a transitive dependency of `syn` and `quote`), adding it as a direct dependency adds no extra compilation overhead. It just makes the dependency explicit and avoids relying on a private API path.
