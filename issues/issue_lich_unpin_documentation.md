# Issue: `Lich<T>` Does Not Implement `Unpin` Bound Documentation

## Summary

`Lich<T>` is automatically `Unpin` (since it only contains `NonNull<T>` and `NonNull<AtomicU32>`, which are `Unpin`). However, this important property is not documented. Users working with `Pin<Lich<T>>` in async contexts need to know that `Lich` is always `Unpin`, meaning they can freely move it even when pinned.

This is distinct from `Soul<T>`, which is `!Unpin` due to `PhantomPinned`.

## Why This Is an Issue

When users combine `phylactery` with async runtimes, they might wonder whether `Lich<dyn Future<Output = T>>` can be polled after moving. Since `Lich` is `Unpin`, it can be freely moved, and `Pin::new(&mut lich)` works without any `unsafe` code.

However, there's a subtlety: while the `Lich` itself is `Unpin`, the underlying `Future` pointed to by the `Lich` might not be. Since `Lich` only provides `&T` access (not `&mut T` or `Pin<&mut T>`), you can't actually poll a `dyn Future` through a `Lich`. But this is worth documenting to prevent confusion.

## Impact

- **Severity**: Very Low (documentation)
- **No correctness issue**.
- **Affected users**: Users combining `Lich` with async code.

## Proposed Fix

Add documentation to the `Lich` type:

```rust
/// # Pinning
///
/// `Lich<T>` is always [`Unpin`], regardless of `T`. This is because `Lich`
/// stores a raw pointer, not the value itself. The actual value is pinned
/// inside the [`Soul`].
///
/// Note that while `Lich` is `Unpin`, it only provides shared access (`&T`)
/// via [`Deref`]. This means that types requiring `Pin<&mut Self>` (such as
/// [`Future`]) cannot be used through a `Lich`.
```

## Files to Modify

- `phylactery/src/lich.rs` (type-level documentation): Add pinning/Unpin documentation.

## Verification

- Documentation renders correctly: `cargo doc --all-features --no-deps`.

## Notes

This complements the existing documentation about `Send` and `Sync` bounds on `Lich`. The auto-trait implementations for `Lich` are:
- `Send` where `for<'a> &'a T: Send` (explicit impl)
- `Sync` where `for<'a> &'a T: Sync` (explicit impl)
- `Unpin` always (automatic)
