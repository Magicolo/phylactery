# Issue 12: `Lich` Missing Common Smart-Pointer Trait Implementations

## Summary

`Lich<T>` behaves like a `&'static T` (a shared reference with a
dynamically-verified lifetime), yet it implements far fewer traits than other
smart-pointer types in the Rust standard library such as `Arc<T>`, `Box<T>`,
and `Rc<T>`.  Missing trait implementations force downstream users to work
around gaps and reduce the ergonomics of the type.

## Location

`phylactery/src/lich.rs` – the list of `impl` blocks currently covers: `Clone`,
`Send`, `Sync`, `Borrow<T>`, `Deref`, `AsRef<T>`, and `Drop`.

## Missing Implementations

### `fmt::Pointer`

`Lich` holds a raw data pointer.  Implementing `fmt::Pointer` allows users (and
`#[derive]`) to print the address of the pointed-to data:

```rust
impl<T: ?Sized> fmt::Pointer for Lich<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.value.as_ptr(), f)
    }
}
```

### `fmt::Display` (where `T: fmt::Display`)

```rust
impl<T: fmt::Display + ?Sized> fmt::Display for Lich<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.data_ref(), f)
    }
}
```

### `PartialEq` / `Eq` (where `T: PartialEq`)

```rust
impl<T: PartialEq + ?Sized> PartialEq for Lich<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data_ref() == other.data_ref()
    }
}
impl<T: Eq + ?Sized> Eq for Lich<T> {}
```

### `PartialOrd` / `Ord` (where `T: PartialOrd`)

```rust
impl<T: PartialOrd + ?Sized> PartialOrd for Lich<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        PartialOrd::partial_cmp(self.data_ref(), other.data_ref())
    }
}
impl<T: Ord + ?Sized> Ord for Lich<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Ord::cmp(self.data_ref(), other.data_ref())
    }
}
```

### `Hash` (where `T: Hash`)

```rust
impl<T: core::hash::Hash + ?Sized> core::hash::Hash for Lich<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.data_ref().hash(state);
    }
}
```

### `fmt::Debug` (tracked separately in issue #05)

Already covered by Issue 05.

## Why These Impls Matter

| Trait        | Without impl                                                  |
|--------------|---------------------------------------------------------------|
| `Display`    | `println!("{}", lich)` fails to compile if T: Display        |
| `PartialEq`  | `lich1 == lich2` fails; can't put Lich in a `HashSet`        |
| `Ord`        | Can't sort or use `BTreeSet<Lich<T>>`                         |
| `Hash`       | Can't use `Lich<T>` as a `HashMap` key                        |
| `Pointer`    | Can't print raw pointer address for debugging                 |

These are the standard trait set that every smart pointer in the Rust ecosystem
provides.  Compare: `Arc<T>` and `Box<T>` implement all of the above.

## Compatibility Notes

- All proposed implementations forward to the underlying `&T`, consistent with
  how `Arc<T>` and `Box<T>` implement these traits.
- `PartialEq` compares **values**, not pointer addresses.  This is the
  idiomatic choice (same as `Box<T>`).  If pointer-identity equality is desired
  in some context, users can compare `Lich` values via `ptr::eq`.

## Plan to Fix

1. Add implementations in `phylactery/src/lich.rs` for each trait listed above,
   behind appropriate `where T: Trait` bounds.
2. Add tests to `phylactery/tests/binding.rs` (or a new `traits.rs`) that
   verify each impl compiles and produces correct results.
3. Ensure all new impls are gated with `#[cfg(feature = "std")]` only if they
   require `std`-only types.  The traits listed above are all in `core`, so no
   feature gate is needed.
