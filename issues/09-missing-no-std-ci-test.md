# Issue 09: Missing `no_std` CI Test

## Summary

The library advertises `#[no_std]` support (via `#![cfg_attr(not(feature =
"std"), no_std)]` and the crate documentation).  However, the CI workflow does
not include a step that compiles the library with `default-features = false` to
verify that the `no_std` build actually works.  A regression that silently
breaks `no_std` compatibility would not be caught.

## Location

- `.github/workflows/test.yml` – the `test` and `clippy` jobs
- `phylactery/Cargo.toml` – `[features]` section

## Current CI Workflow (relevant excerpt)

```yaml
test:
  steps:
    - run: cargo hack nextest run --release --all-targets --verbose --feature-powerset --no-tests pass
    - run: cargo hack test --doc --release --all-features --verbose

clippy:
  steps:
    - run: cargo hack clippy --release --all-targets --verbose --feature-powerset -- --deny warnings …
```

`cargo hack --feature-powerset` generates all combinations of the declared
features, which *does* include the empty feature set (i.e., no `std`, no
`shroud`).  However:

1. The `--all-targets` flag includes `tests/` and `examples/` which use
   `std::thread`, `std::sync::Mutex`, etc.  These targets fail to compile under
   `no_std`, so `cargo hack` skips or errors on them in the no-feature
   combination — but only if `--no-tests pass` does not mask the failure.
2. There is no dedicated job that explicitly passes `--target thumbv7m-none-eabi`
   (or another bare-metal target) to confirm the library compiles without the
   standard library on a target that has no std.
3. There is no check that `phylactery_macro` (a proc-macro crate compiled for
   the host) still works when used from a `no_std` context.

## Why This Is an Issue

- A future contributor might accidentally add a dependency on `std` types
  (e.g., `std::collections::HashMap`, `std::string::String`) outside of a
  `#[cfg(feature = "std")]` guard.  Without a dedicated `no_std` test, this
  regression would ship to users.
- The README explicitly lists `no_std` as a feature:
  > *This library supports `#[no_std]` (use `default-features = false` in your
  > 'Cargo.toml').*
- The crate keywords include `no-std` (`categories = ["no-std", …]`).

## Plan to Fix

### Option A: Add a bare-metal cross-compilation check to CI

Add a new CI job (or a step in the existing `test` job) that compiles the
library for a `no_std` target:

```yaml
no_std:
  runs-on: ubuntu-latest
  container:
    image: ghcr.io/magicolo/rust
  steps:
    - uses: actions/checkout@v5
    - run: rustup target add thumbv7m-none-eabi
    - run: cargo build --target thumbv7m-none-eabi --no-default-features
    - run: cargo build --target thumbv7m-none-eabi --no-default-features --features shroud
```

`thumbv7m-none-eabi` is a popular bare-metal Cortex-M target with no `std`.

### Option B: Use `cargo check` to keep CI fast

If a full cross-compilation environment is not easily available in the container
image, `cargo check` is sufficient to verify that the code compiles:

```yaml
- run: cargo check --target thumbv7m-none-eabi --no-default-features
- run: cargo check --target thumbv7m-none-eabi --no-default-features --features shroud
```

### Additional: Guard the test/example files

The test file `phylactery/tests/binding.rs` imports from `std` unconditionally:

```rust
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    thread::{sleep, spawn},
};
```

These should be gated behind `#[cfg(feature = "std")]` or the file should be
annotated to be excluded from `no_std` feature combinations:

```rust
#![cfg(feature = "std")]
```

Without this guard, `cargo hack --feature-powerset --all-targets` will include
the test in the `no_std` build and fail to compile it.
