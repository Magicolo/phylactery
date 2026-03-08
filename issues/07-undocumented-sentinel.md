# Issue 07: Undocumented `u32::MAX` Sentinel in `bindings()` and `sever()`

## Summary

The `count` field in `Soul` (and the corresponding pointer in `Lich`) is used
both as a live-binding counter **and** as a "severed" sentinel value: when
`sever` succeeds it atomically writes `u32::MAX` to signal that the `Soul` is
dead.  The `bindings()` method silently maps `u32::MAX → 0` via a non-obvious
wrapping-arithmetic trick.  None of this is documented.  The sentinel state and
the arithmetic trick are invisible to anyone reading the code without digging
through the full implementation.

## Location

- `phylactery/src/soul.rs` – `Soul::bindings`, lines 94-99; `sever`, line 192
- `phylactery/src/lich.rs` – `Lich::bindings`, lines 40-45

## Current Code

```rust
// soul.rs — bindings()
pub fn bindings(&self) -> usize {
    self.count
        .load(Ordering::Relaxed)
        .wrapping_add(1)
        .saturating_sub(1) as _
}
```

```rust
// lich.rs — same trick
pub fn bindings(&self) -> usize {
    self.count_ref()
        .load(Ordering::Relaxed)
        .wrapping_add(1)
        .saturating_sub(1) as _
}
```

```rust
// soul.rs — sever writes u32::MAX
count.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed)
```

## Why This Is an Issue

### The trick is not self-documenting

`wrapping_add(1).saturating_sub(1)` is a compact way of writing:

```
if value == u32::MAX { 0 } else { value }
```

but this equivalence is not obvious to a reader, and the reason for mapping
`u32::MAX` to `0` (i.e., "the Soul has been severed, treat it as having zero
bindings") is nowhere explained.

### The sentinel state itself is undocumented

There is no mention in any doc comment that:

1. The `count` field doubles as a "dead" marker.
2. The value `u32::MAX` means "Soul has been severed."
3. `bindings()` returns `0` both for an unsevered Soul with no Liches AND for
   a severed Soul.

This ambiguity can mislead callers:

```rust
let soul = Box::pin(Soul::new(|| {}));
let soul = Soul::sever(soul);  // severs; count is now u32::MAX
// Soul is no longer pinned, but…
// bindings() would return 0 — identical to an unsevered, unbound Soul.
```

### Confusion in `increment`

`increment` (used by `Soul::bind`) guards against the sentinel:

```rust
Err(u32::MAX) => unreachable!(),
```

This `unreachable!()` would panic if called after sever, but the comment
says it is unreachable — implying a safety invariant that is never stated
explicitly.

### Dead match arm in `sever`

```rust
Ok(0 | u32::MAX) | Err(u32::MAX) => break true,
```

`Ok(u32::MAX)` can never be reached: `compare_exchange(0, u32::MAX, …)` only
returns `Ok(old_value)` when the old value equals the first argument (`0`), so
`Ok(u32::MAX)` would require the old value to be both `0` and `u32::MAX`
simultaneously — impossible.  The arm is dead code, but no comment explains
this, and the compiler does not warn on it.

## Plan to Fix

1. **Add a named constant** for the sentinel value:

```rust
// soul.rs or a shared location
/// Sentinel value written to `Soul.count` by `sever` to indicate that the
/// Soul has been permanently deactivated.  `u32::MAX - 1` is the maximum
/// number of live Liches; `u32::MAX` is reserved as the dead state.
const SEVERED: u32 = u32::MAX;
```

2. **Document `bindings()`** to explain what it returns in the severed state:

```rust
/// Returns the number of [`Lich`]es currently bound to this [`Soul`].
///
/// Returns `0` both when no Liches are bound and when the [`Soul`] has
/// already been severed.
pub fn bindings(&self) -> usize { … }
```

3. **Replace the wrapping trick** with an explicit branch, or at least add an
   inline comment:

```rust
pub fn bindings(&self) -> usize {
    let raw = self.count.load(Ordering::Relaxed);
    // u32::MAX is the "severed" sentinel; treat it as 0 live bindings.
    raw.wrapping_add(1).saturating_sub(1) as _
}
```

4. **Mark the dead arm as unreachable with a comment**:

```rust
// `compare_exchange(0, …)` returns Ok(old_value) only when old_value == 0,
// so Ok(u32::MAX) is structurally impossible; only Ok(0) can appear here.
Ok(0) | Err(u32::MAX) => break true,
```

   (Remove `Ok(u32::MAX)` from the pattern to make the dead arm visible as a
   compiler lint if the match arm ever does become reachable.)

5. **Document the `increment` unreachable arm**:

```rust
// Err(u32::MAX) means sever has already been called.  `bind` requires a
// Pin<&Self> which is impossible to hold after sever consumes the Pin,
// so this branch is unreachable in safe code.
Err(u32::MAX) => unreachable!("bind called on a severed Soul"),
```
