# Issue: Missing `no_std` Compilation Test in CI

## Summary

The CI pipeline (`test.yml`) uses `cargo hack` to test all feature combinations, but does not explicitly test compilation on a `no_std` target (e.g., `thumbv6m-none-eabi`). The crate has `#![cfg_attr(not(feature = "std"), no_std)]`, and while the `--feature-powerset` flag tests without `std`, it still compiles for the host target which has `std` available. A true `no_std` target would catch issues like accidentally importing from `std` in `no_std` mode.

## Why This Is an Issue

The current CI commands include:

```yaml
- cargo hack nextest run --release --all-targets --verbose --feature-powerset --no-tests pass
```

This tests all feature combinations (including `--no-default-features` which disables `std`), but on the host target (`x86_64-unknown-linux-gnu`). Even without `std` feature, the host target has `std` available as a crate, so some `std` imports might accidentally compile.

A true `no_std` compilation test would use a bare-metal target:

```bash
rustup target add thumbv6m-none-eabi
cargo build --target thumbv6m-none-eabi --no-default-features
cargo build --target thumbv6m-none-eabi --no-default-features --features shroud
```

## Impact

- **Severity**: Low (CI completeness)
- **Risk**: A future change might accidentally introduce an `std` dependency in `no_std` mode, which wouldn't be caught by CI.

## Proposed Fix

Add a `no_std` compilation check to the CI matrix:

```yaml
matrix:
    command:
        # ... existing commands ...
        - rustup target add thumbv6m-none-eabi && cargo build --target thumbv6m-none-eabi --no-default-features -p phylactery
        - rustup target add thumbv6m-none-eabi && cargo build --target thumbv6m-none-eabi --no-default-features --features shroud -p phylactery
```

Alternatively, this could be a separate job that installs the target first.

## Files to Modify

- `.github/workflows/test.yml`: Add `no_std` compilation test to the matrix.

## Verification

- The new CI steps should pass without errors.
- Verify that the `atomic-wait` crate's `no_std` fallback compiles on bare-metal targets.

## Notes

The `atomic-wait` dependency claims `no_std` support, but its behavior on bare-metal targets (where there's no OS futex) falls back to spin-waiting. This is documented separately in `issue_no_std_blocking_docs.md`.

Note that `atomic-wait` version 1.1 requires careful testing on `thumbv6m-none-eabi` because it depends on `libc` for futex operations. In `no_std` mode without OS, it must use a pure spin-wait fallback. This should be verified.
