# Issue 04: `Lich::data_ref` Returns an Unnecessary `Result`

## Summary

The private helper method `Lich::data_ref` always returns `Ok(…)` and can
never return `Err(…)`.  Wrapping the return value in `Result<&T, &'static str>`
is misleading (it implies a possible failure path that does not exist), makes
all call sites use `.unwrap()`, and is flagged by `clippy::unnecessary_wraps`.

## Location

`phylactery/src/lich.rs`, lines 53-57.

```rust
fn data_ref(&self) -> Result<&T, &'static str> {
    // Safety: the pointers are valid for the lifetime of `self`; guaranteed by the
    // reference count.
    Ok(unsafe { self.value.as_ref() })
}
```

## Why This Is an Issue

### Misleading API

Returning `Result` signals to readers that the operation can fail.  A reader
unfamiliar with the codebase may wonder:
- What error can occur?
- Under what conditions does it return `Err`?
- Why are all call sites blindly calling `.unwrap()`?

None of these questions have meaningful answers because the function never
returns `Err`.

### Clippy warning

Running `cargo clippy -- -W clippy::pedantic` produces:

```
warning: this function's return value is unnecessarily wrapped by `Result`
  --> phylactery/src/lich.rs:53:5
   |
53 |     fn data_ref(&self) -> Result<&T, &'static str> {
```

### Noisy call sites

```rust
impl<T: ?Sized> Borrow<T> for Lich<T> {
    fn borrow(&self) -> &T {
        self.data_ref().unwrap()   // unwrap that can never panic
    }
}
// … same in Deref and AsRef impls
```

### Possible historical context

The `Result` wrapper may be a leftover from an earlier design where `Lich`
could become invalid (e.g., after sever), in which case it might have returned
`Err`.  The current design relies on the Soul's `Drop` blocking until all
Liches are dropped, making a Lich invalid state impossible.

## Plan to Fix

1. Change the return type of `data_ref` to `&T` and remove the `Ok` wrapper:

```rust
fn data_ref(&self) -> &T {
    // Safety: the pointer is valid for the lifetime of `self`; guaranteed by
    // the reference count in the associated Soul.
    unsafe { self.value.as_ref() }
}
```

2. Remove `.unwrap()` from all call sites:

```rust
impl<T: ?Sized> Borrow<T> for Lich<T> {
    fn borrow(&self) -> &T {
        self.data_ref()
    }
}
impl<T: ?Sized> Deref for Lich<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.data_ref()
    }
}
impl<T: ?Sized> AsRef<T> for Lich<T> {
    fn as_ref(&self) -> &T {
        self.data_ref()
    }
}
```

3. Optionally, since `data_ref` is now trivial and private, consider inlining
   it into call sites or marking it `#[inline(always)]`.

4. Verify with `cargo clippy` that the warning is gone and all tests pass.
