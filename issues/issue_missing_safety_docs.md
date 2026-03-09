# Issue: Missing `# Safety` Documentation on Internal Unsafe Functions and Operations

## Summary

Several internal unsafe functions and unsafe blocks in the `phylactery` crate lack formal `# Safety` documentation sections. While most have inline comments explaining safety, the Rust API Guidelines and the Nomicon recommend that every `unsafe fn` has a `# Safety` doc section describing:
1. What invariants the caller must uphold.
2. What could go wrong if those invariants are violated.

## Current State

### Functions needing `# Safety` documentation:

#### 1. `Lich::redeem_unchecked` (`lich.rs:72-81`)

Current:
```rust
/// Safety: must be called only once for this `Lich` when it became
/// unreachable.
unsafe fn redeem_unchecked(&self) -> usize { ... }
```

The comment uses `/// Safety:` but should use the standard `/// # Safety` heading format:
```rust
/// # Safety
///
/// Must be called only once for this `Lich`, and only when it has become
/// unreachable (i.e., is being dropped or consumed by `redeem`).
/// Calling this more than once will cause an underflow of the reference
/// count, leading to undefined behavior (premature sever/data access after free).
```

#### 2. `Soul::unpin` (`soul.rs:157-168`)

Current:
```rust
/// # Safety
///
/// The caller must ensure that `sever` (the standalone free function in
/// this module) has returned `true` for this Soul's `count` field
/// before calling this function.  That is, all bound [`Lich`]es must
/// have been dropped and the `count` must have been atomically set to
/// `u32::MAX`.
unsafe fn unpin<S: Deref<Target = Self>>(this: Pin<S>) -> S { ... }
```

This is well-documented. ✓

#### 3. `Shroud::shroud` (`shroud.rs:10-12`)

The trait method has no safety documentation at all:
```rust
pub trait Shroud<T: ?Sized> {
    fn shroud(from: NonNull<T>) -> NonNull<Self>;
}
```

While `shroud` is not an `unsafe fn`, its implementations contain `unsafe` blocks that rely on the caller providing valid input. Documentation should describe:
- What `from` must point to (a valid, aligned, dereferenceable allocation).
- What the returned `NonNull<Self>` must satisfy (same allocation, valid metadata).

#### 4. `Soul::value_ptr` and `Soul::count_ptr` (`soul.rs:170-186`)

These have inline comments but not formal `# Safety` sections:
```rust
fn value_ptr(self: Pin<&Self>) -> NonNull<T> {
    // Safety: we use `addr_of!` to obtain a raw pointer to the field without
    // creating an intermediate reference, preserving the raw provenance...
    unsafe { NonNull::new_unchecked(addr_of!(self.value) as _) }
}
```

Since these are private methods (no `pub`), formal `# Safety` docs are less critical. However, the safety comment should explain why the `*const` to `*mut` cast is valid.

### Unsafe blocks needing better safety justification:

#### 5. `into_value` (`soul.rs:81-85`)

```rust
pub fn into_value(self) -> T {
    // No need to run `<Soul as Drop>::drop` since no `Lich` can be bound, given by
    // the fact that this `Soul` is unpinned.
    unsafe { read(&ManuallyDrop::new(self).value) }
}
```

The comment explains the reasoning, but doesn't explain why `read` is safe here. A more complete safety comment would be:

```rust
// Safety:
// 1. `ManuallyDrop::new(self)` consumes `self` without running its Drop.
// 2. `read` copies the value out of the ManuallyDrop without dropping the
//    original. Since ManuallyDrop's destructor is a no-op, the Soul's
//    memory is leaked (but since Soul only owns the value and a counter,
//    this is a controlled leak of the AtomicU32 and PhantomPinned, which
//    have no Drop impls).
// 3. This is safe because no Lich can exist (the Soul is unpinned).
```

## Impact

- **Severity**: Low (documentation quality)
- **No runtime behavior change**.
- **Benefit**: Makes safety reasoning explicit and auditable, helps future contributors understand the safety story.

## Proposed Fix

1. Update `Lich::redeem_unchecked` to use standard `# Safety` heading format.
2. Add `# Safety` documentation to `Shroud::shroud` trait method (or add it to the trait-level doc).
3. Enhance safety comments in `into_value` to be more explicit.
4. Optionally: add safety comments to `value_ptr` and `count_ptr` explaining the `*const -> *mut` cast.

## Files to Modify

- `phylactery/src/lich.rs` (lines 72-73): Fix `# Safety` heading for `redeem_unchecked`.
- `phylactery/src/shroud.rs` (lines 10-12): Add documentation to `Shroud` trait.
- `phylactery/src/soul.rs` (lines 79-84): Enhance `into_value` safety comment.

## Verification

- `cargo doc --all-features --no-deps` must succeed.
- Documentation renders correctly.
