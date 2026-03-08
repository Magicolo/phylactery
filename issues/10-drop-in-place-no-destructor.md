# Issue 10: Unnecessary `drop_in_place` on a No-Destructor Type in `Soul::consume`

## Summary

`Soul::consume` calls `drop_in_place(&mut soul.count)` where `soul.count` is
an `AtomicU32`, a type that implements `Copy` and has a trivial destructor (no
`Drop` impl).  The call is a no-op but adds noise, and the pattern of manually
driving drops inside a `ManuallyDrop` is fragile if the struct layout ever
changes (e.g., if a new field with a real destructor is added between `count`
and `value`).

## Location

`phylactery/src/soul.rs`, `Soul::consume`, lines 65-71.

```rust
pub fn consume(self) -> T {
    // No need to run `<Soul as Drop>::drop` since no `Lich` can be bound, given by
    // the fact that this `Soul` is unpinned.
    let mut soul = ManuallyDrop::new(self);
    unsafe { drop_in_place(&mut soul.count) };   // <-- no-op: AtomicU32 has no destructor
    unsafe { read(&soul.value) }
}
```

## Why This Is an Issue

### `AtomicU32` has no destructor

`AtomicU32` is `Copy` and does not implement `Drop`, so `drop_in_place` on it
is guaranteed to be a no-op.  The call is misleading because it implies that
`count` requires explicit cleanup, which it does not.

### `_marker: PhantomPinned` is not dropped

`PhantomPinned` also has no destructor and is a zero-sized type.  The `consume`
function drops `count` explicitly but not `_marker`, which is inconsistent.
Neither needs to be explicitly dropped, so dropping one and not the other
creates a false impression about what requires cleanup.

### Fragility if the struct evolves

If a future version of `Soul<T>` adds a field with a real destructor (e.g., an
`Arc`, a `Box`, or a custom `Drop` impl), a developer following the pattern of
`Soul::consume` must remember to add another `drop_in_place` call.  Missing one
would silently leak the resource.  The correct pattern is either:
- Use `ManuallyDrop::new` and only `read` the field that needs to be moved out,
  leaving all other fields to be dropped normally.
- Use `ptr::read` on each field and let them drop at the end of scope.

### The comment is slightly misleading

The comment says:
> *No need to run `<Soul as Drop>::drop` since no `Lich` can be bound, given by
> the fact that this `Soul` is unpinned.*

This explains why the `Drop` impl is bypassed, but it does not explain why
`drop_in_place` is called separately.  A reader might conclude that
`AtomicU32::drop` is important, when in fact it is not.

## Plan to Fix

Remove the `drop_in_place` call and add a clarifying comment:

```rust
pub fn consume(self) -> T {
    // Skip `<Soul as Drop>::drop` (which would call `sever`) because no Lich
    // can be bound to an unpinned Soul — `bind` requires `Pin<&Self>`.
    // `_marker` (PhantomPinned) and `count` (AtomicU32) have trivial
    // destructors and do not need explicit cleanup.
    let soul = ManuallyDrop::new(self);
    // Safety: `soul` is wrapped in `ManuallyDrop`, so its fields will not be
    // double-dropped.  We move `value` out; `count` and `_marker` are
    // trivially-destructible and are silently discarded with the ManuallyDrop.
    unsafe { read(&soul.value) }
}
```

If a future field with a non-trivial destructor is added, the compile-time
check (Miri / `clippy::mem_forget` lint) will catch leaks.  Alternatively,
restructure `consume` to use `into_parts` decomposition if the struct grows:

```rust
pub fn consume(self) -> T {
    let soul = ManuallyDrop::new(self);
    // SAFETY: ManuallyDrop prevents the outer struct from running Drop,
    // so we can safely move out the inner value.
    unsafe { ManuallyDrop::new(ptr::read(&soul.value)).into_inner() }
}
```

After applying the change, run `cargo test` and `cargo miri test` to confirm
no regressions.
