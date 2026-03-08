# Issue 19: `Lich::drop` Calls `count_ref()` Twice Redundantly

## Summary

In `Lich::drop`, `count_ref()` is called twice: once to store the reference in
a local variable `count`, and again immediately as an argument to `decrement`.
The second call is redundant — the local `count` already holds the reference.
This is a minor code-quality issue that creates unnecessary noise and may
confuse readers into thinking the two calls serve different purposes.

## Location

`phylactery/src/lich.rs`, `<Lich as Drop>::drop`, lines 90-97.

```rust
impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        let count = self.count_ref();           // call 1
        if decrement(self.count_ref()) == 0 {   // call 2 — redundant
            atomic_wait::wake_one(count);
        }
    }
}
```

## Why This Is an Issue

- The two calls appear to suggest that there might be a reason to have two
  separate borrows (e.g., that the second one could return a different value),
  when in fact they both return `&self.count`.
- The first call stores `count` but it is only used in `wake_one`; the second
  call to `count_ref()` for `decrement` bypasses the already-stored `count`.
- A reader unfamiliar with the code might wonder: "Why is `count` stored in a
  local variable if it isn't used in `decrement`?"

## Plan to Fix

Use the already-stored `count` reference for the `decrement` call:

```rust
impl<T: ?Sized> Drop for Lich<T> {
    fn drop(&mut self) {
        let count = self.count_ref();
        if decrement(count) == 0 {
            atomic_wait::wake_one(count);
        }
    }
}
```

This eliminates the second `count_ref()` call and makes it obvious that `count`
is stored specifically to be used in both `decrement` and the conditional
`wake_one`.

Note: This is a pure refactor with zero semantic change; the generated machine
code should be identical.  Verify with `cargo test` and
`cargo +nightly miri test --all-features` after the change.
