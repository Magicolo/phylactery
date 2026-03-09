# Issue 16: `Soul` Missing Common Standard-Library Trait Implementations

## Summary

`Soul<T>` derives `Debug` and manually implements `Deref<Target=T>`,
`AsRef<T>`, and `Borrow<T>`.  However, it is missing several other
trait implementations that users would expect from a smart-pointer-like wrapper
type.  This creates an inconsistent API and reduces ergonomics for downstream
users.

## Location

`phylactery/src/soul.rs` – the `impl` blocks for `Soul<T>`.

## Missing Implementations

### `Default` (where `T: Default`)

`Box<T>`, `Arc<T>`, and virtually every wrapper type in the standard library
implement `Default` when `T: Default`.  `Soul` does not:

```rust
impl<T: Default> Default for Soul<T> {
    fn default() -> Self {
        Soul::new(T::default())
    }
}
```

### `fmt::Display` (where `T: fmt::Display`)

`Box<T>` implements `Display` when `T: Display`.  `Soul` only has `Deref` but
no `Display`:

```rust
impl<T: fmt::Display + ?Sized> fmt::Display for Soul<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}
```

### `From<T>`

Ergonomic construction via the `From` / `Into` mechanism:

```rust
impl<T> From<T> for Soul<T> {
    fn from(value: T) -> Self {
        Soul::new(value)
    }
}
```

### `PartialEq` / `Eq` / `PartialOrd` / `Ord` (where `T: Trait`)

Comparing `Soul` values by their contained data is natural:

```rust
impl<T: PartialEq + ?Sized> PartialEq for Soul<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<T: Eq + ?Sized> Eq for Soul<T> {}
```

Similarly for `PartialOrd` / `Ord`.

### `Hash` (where `T: Hash`)

```rust
impl<T: core::hash::Hash + ?Sized> core::hash::Hash for Soul<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}
```

### `DerefMut` (where `T: Sized`)

This is more controversial: `Soul` holds `T` behind a pin, but before pinning,
a `Soul<T>` is just a wrapper.  The `value` field is private, so users must go
through `Deref` to read the value.  Exposing `DerefMut` would let callers
mutate `value` before any Liches are created, which is sound.  However,
`DerefMut` after `Lich`es have been bound would allow mutating shared data,
which is also sound since `Lich` only provides `&T` (shared reference) — but
potentially surprising.

For `Soul<T: ?Sized>` this is **not** implementable (unsized types have no safe
mutable access pattern via Deref), so this applies only to `Soul<T: Sized>`.

## Why These Are Missing

The current implementation focuses on the core safety mechanism (`Deref`,
`Drop`, `bind`, `sever`) and the trait impls that `Lich` depends on (`Borrow`).
The general-purpose ergonomic impls were likely not added yet.

## Impact

```rust
// Can't use Soul with serde's Default bound:
// requires `Soul<T>: Default` where `T: Default`

// Can't compare two Souls:
let s1 = Soul::new(42u32);
let s2 = Soul::new(42u32);
// s1 == s2  // compile error

// Can't print a Soul that wraps a Display type:
let s = Soul::new("hello");
println!("{s}");  // compile error: Soul<&str> doesn't implement Display
```

## Plan to Fix

Add the following implementations in `soul.rs`:

1. `Default for Soul<T: Default>` — call `Soul::new(T::default())`
2. `From<T> for Soul<T>` — call `Soul::new(value)`
3. `Display for Soul<T: Display + ?Sized>` — delegate to `T`
4. `PartialEq<Soul<T>> for Soul<T: PartialEq + ?Sized>` — compare values
5. `Eq for Soul<T: Eq + ?Sized>`
6. `PartialOrd for Soul<T: PartialOrd + ?Sized>` — compare values
7. `Ord for Soul<T: Ord + ?Sized>` — compare values
8. `Hash for Soul<T: Hash + ?Sized>` — hash the value

Each implementation should delegate to the corresponding impl on `self.value`.

Add tests in `tests/binding.rs` (or a new file) verifying the new impls
compile and produce correct results.

9. `DerefMut for Soul<T>` (Sized only) — delegate to `self.value`

Consider also adding these impls for `Lich<T>` in a separate change (Issue 12).
