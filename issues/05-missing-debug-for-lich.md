# Issue 05: `Lich<T>` Does Not Implement `Debug`

## Summary

`Soul<T>` derives `Debug`, but `Lich<T>` has no `Debug` implementation at all.
This asymmetry makes debugging harder – printing a `Soul` works, but trying to
print a `Lich` fails to compile.

## Location

`phylactery/src/lich.rs` – the `Lich<T>` struct definition and impls, which
cover `Clone`, `Borrow`, `Deref`, `AsRef`, and `Drop`, but not `Debug`.

## Why This Is an Issue

### Compile error when trying to debug-print a `Lich`

```rust
let soul = Box::pin(Soul::new(|| 'a'));
let lich = soul.as_ref().bind::<dyn Fn() -> char>();
println!("{lich:?}"); // compile error: `Lich<dyn Fn() -> char>` doesn't implement `Debug`
```

### Asymmetry with `Soul`

`Soul<T: Debug>` derives `Debug` automatically.  There is no reason `Lich<T>`
should not have a matching implementation.

### Impact on downstream users

Any struct that contains a `Lich<T>` and derives `Debug` will fail to compile:

```rust
#[derive(Debug)]  // ERROR: Lich<dyn Fn()> does not implement Debug
struct MyState {
    handler: Lich<dyn Fn()>,
}
```

This forces users to manually implement `Debug` for any struct containing a
`Lich`, which is unexpected and frustrating.

## What a Good `Debug` Implementation Should Show

A useful `Debug` output for `Lich` should include:
- The value it points to (if `T: Debug`) – reachable via `self.data_ref()`.
- Optionally the current binding count, so users can see how many `Lich`es
  refer to the same `Soul`.

Example output: `Lich { value: <value>, bindings: 3 }`

## Plan to Fix

Add an explicit `Debug` implementation in `lich.rs`:

```rust
use core::fmt;

impl<T: fmt::Debug + ?Sized> fmt::Debug for Lich<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lich")
            .field("value", self.data_ref())
            .field("bindings", &self.bindings())
            .finish()
    }
}
```

Notes:
- The `T: Debug` bound mirrors what `#[derive(Debug)]` generates for `Soul<T>`.
- `self.data_ref()` already provides `&T`, which `f.debug_struct` can use
  directly.
- If showing the value is not desired (e.g., for privacy), an alternative is to
  show only the raw pointer address as in `fmt::Pointer`, but that would be less
  useful than showing the value.

**Additional related missing trait implementations** worth considering in the
same patch:

| Trait                       | Condition   | Notes                               |
|-----------------------------|-------------|-------------------------------------|
| `fmt::Pointer`              | none        | Show the raw data pointer address   |
| `PartialEq` / `Eq`          | `T: PartialEq + ?Sized` | Compare underlying values |
| `PartialOrd` / `Ord`        | `T: PartialOrd + ?Sized` | Order by underlying values |
| `Hash`                      | `T: Hash + ?Sized` | Forward to underlying value    |
| `Display`                   | `T: fmt::Display + ?Sized` | Forward display       |

These are not strictly required but follow the principle of least surprise for a
smart-pointer-like type (compare `Box<T>`, `Arc<T>`, etc., which implement all
of the above).
