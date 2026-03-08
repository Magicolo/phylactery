# Issue 06: Missing `#[must_use]` Attributes on Key Methods

## Summary

Several public methods in `Soul` and `Lich` return meaningful values that
should not be silently discarded.  None of them are annotated with
`#[must_use]`, so the compiler does not warn when the caller ignores the return
value.  Ignoring these return values can result in subtle bugs (e.g., a freshly
created `Lich` that is immediately dropped, or a failed `redeem` that goes
unnoticed).

## Location

`phylactery/src/soul.rs` and `phylactery/src/lich.rs` – public method
signatures.

## Affected Methods

### `Soul::bind` — `phylactery/src/soul.rs:79`

```rust
pub fn bind<S: Shroud<T> + ?Sized>(self: Pin<&Self>) -> Lich<S>
```

If the returned `Lich` is discarded, it is immediately dropped, decrementing
the counter and invoking `wake_one` for no reason.  The call is a no-op and
almost certainly a programmer mistake.

Clippy already flags this as `missing_const_for_fn` (for `count_ref` /
`data_ref`); a `#[must_use]` warning on `bind` would catch a related class of
error.

### `Soul::redeem` — `phylactery/src/soul.rs:108`

```rust
pub fn redeem<S: ?Sized>(&self, lich: Lich<S>) -> Result<usize, Lich<S>>
```

The `Err` variant carries the `Lich` back to the caller.  If the caller ignores
the `Result`, the `Lich` is silently dropped inside `Result`'s destructor.
From the caller's perspective, the Lich was "redeemed" but actually wasn't.
`Result` in Rust already has `#[must_use]` on the type itself, but an
additional `#[must_use = "…"]` on the method provides a more actionable
diagnostic.

### `Soul::try_sever` — `phylactery/src/soul.rs:130`

```rust
pub fn try_sever<S: Deref<Target = Self>>(this: Pin<S>) -> Result<S, Pin<S>>
```

Ignoring the `Result` means the caller never finds out whether the sever
succeeded, and the `Soul` is dropped without the caller noticing.

### `Soul::consume` — `phylactery/src/soul.rs:65`

```rust
pub fn consume(self) -> T
```

Discarding the returned `T` means the value is quietly dropped.  While this is
not necessarily wrong, it is almost never intentional.  This is flagged by
`clippy::must_use_unit` / `clippy::missing-must-use-unit` in some pedantic
modes.

### `Soul::bindings` — `phylactery/src/soul.rs:94`

```rust
pub fn bindings(&self) -> usize
```

Calling `bindings()` purely for its side effects makes no sense – there are
none.  Silently discarding the count is almost certainly a bug.

### `Lich::bindings` — `phylactery/src/lich.rs:40`

```rust
pub fn bindings(&self) -> usize
```

Same rationale as `Soul::bindings`.

### `Soul::is_bound` — `phylactery/src/soul.rs:88`

```rust
pub fn is_bound<S: ?Sized>(&self, lich: &Lich<S>) -> bool
```

Calling a predicate and ignoring the boolean is always a bug.

## Plan to Fix

Add `#[must_use]` (optionally with a descriptive message) to each method listed
above.

```rust
// soul.rs

#[must_use = "the Lich is immediately dropped if not used"]
pub fn bind<S: Shroud<T> + ?Sized>(self: Pin<&Self>) -> Lich<S> { … }

#[must_use = "if Err, the Lich was not redeemed and is returned"]
pub fn redeem<S: ?Sized>(&self, lich: Lich<S>) -> Result<usize, Lich<S>> { … }

#[must_use = "if Err, the Soul has not been severed"]
pub fn try_sever<S: Deref<Target = Self>>(this: Pin<S>) -> Result<S, Pin<S>> { … }

#[must_use = "discarding the value drops it silently"]
pub fn consume(self) -> T { … }

#[must_use]
pub fn bindings(&self) -> usize { … }

#[must_use]
pub fn is_bound<S: ?Sized>(&self, lich: &Lich<S>) -> bool { … }
```

```rust
// lich.rs

#[must_use]
pub fn bindings(&self) -> usize { … }
```

After applying the fix, run `cargo clippy --all-features` and `cargo test` to
confirm no warnings or regressions.  Additionally run `cargo test --doc` to
ensure the doc-test examples still compile (the compiler will warn on any
doc-test that discards a `#[must_use]` value without an explicit `let _`).
