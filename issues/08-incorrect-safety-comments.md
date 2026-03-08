# Issue 08: Incorrect Safety Comment References (`B::sever` Does Not Exist)

## Summary

Two `// Safety:` comments in `soul.rs` reference a non-existent `B::sever`
method.  This appears to be a stale copy-paste from an older version of the
code where the sever logic lived inside a trait method.  The current code uses a
standalone `sever` free function, so the comments are factually incorrect and
misleading to anyone auditing the `unsafe` blocks.

## Location

`phylactery/src/soul.rs`, lines 122 and 132.

```rust
// soul.rs, Soul::sever
if sever::<true>(&this.count) {
    // Safety: all bindings have been severed, guaranteed by `B::sever`.
    unsafe { Self::unpin(this) }
```

```rust
// soul.rs, Soul::try_sever
if sever::<false>(&this.count) {
    // Safety: all bindings have been severed, guaranteed by `B::sever`.
    Ok(unsafe { Self::unpin(this) })
```

## Why This Is an Issue

Rust safety comments are the primary documentation for why a given `unsafe`
block does not invoke undefined behaviour.  Reviewers and auditors rely on them
to understand the safety invariants.

An incorrect reference (`B::sever`) sends auditors chasing a non-existent
symbol, erodes trust in the safety documentation, and makes it harder to verify
the `unsafe` blocks are actually sound.

### The correct reference

The safety is guaranteed by the standalone free function `sever::<FORCE>` (in
the same file, lines 190-198), not by any method `B::sever`.  Specifically:

- `sever::<true>` blocks until `count` transitions from any non-`u32::MAX`
  value to `u32::MAX`, guaranteeing all live Liches have been dropped before
  returning.
- `sever::<false>` only attempts the transition once and returns `false`
  (without blocking) if any Liches remain.

After either variant returns `true`, it is safe to call `Soul::unpin` because
no `Lich` holds a pointer into the Soul's memory.

## Plan to Fix

Update both `// Safety:` comments to reference the correct function and
describe the invariant precisely:

```rust
// soul.rs, Soul::sever
if sever::<true>(&this.count) {
    // Safety: `sever::<true>` returned `true`, which guarantees the atomic
    // count has been set to `u32::MAX` and all previously live Liches have
    // been dropped.  It is therefore safe to unpin the Soul.
    unsafe { Self::unpin(this) }
```

```rust
// soul.rs, Soul::try_sever
if sever::<false>(&this.count) {
    // Safety: `sever::<false>` returned `true`, which means the CAS
    // succeeded (count was 0) and no Liches are bound.  It is therefore
    // safe to unpin the Soul.
    Ok(unsafe { Self::unpin(this) })
```

Additionally, update the `Safety` doc comment on the private `Soul::unpin`
function (line 139-142) to describe the invariant in terms of `sever` rather
than in terms of a fictional `B::sever`:

```rust
/// # Safety
///
/// The caller must ensure that `sever` (the standalone free function in this
/// module) has returned `true` for this Soul's `count` field before calling
/// this function.  That is, all bound [`Lich`]es must have been dropped and
/// the `count` must have been atomically set to `u32::MAX`.
unsafe fn unpin<S: Deref<Target = Self>>(this: Pin<S>) -> S { … }
```
