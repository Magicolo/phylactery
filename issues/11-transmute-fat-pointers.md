# Issue 11: `Shroud` Transmute Between Fat Pointers Should Use Pointer Casts

## Summary

Throughout `phylactery/src/shroud.rs` and the generated code in
`phylactery_macro/src/lib.rs`, lifetime changes on fat (trait-object) pointers
are performed via `core::mem::transmute`.  Clippy flags each of these as
`clippy::transmute_ptr_to_ptr` ("transmute from a pointer to a pointer") and
suggests using `as` casts or `pointer::cast` instead.  While the current
transmutes are *technically sound* (because fat-pointer layout is identical for
different lifetimes of the same trait object), unnecessary `transmute` calls are
harder to audit, suppress future lint improvements, and hide the actual intent
from the compiler.

## Location

- `phylactery/src/shroud.rs` – every `shroud_ty!` and `shroud_fn!` expansion
  (dozens of occurrences)
- `phylactery_macro/src/lib.rs` – the generated `Shroud` implementations
  (lines 65-94)

## Example (from `shroud.rs`, blanket non-dynamic impl)

```rust
fn shroud(from: ::core::ptr::NonNull<TConcrete>) -> ::core::ptr::NonNull<Self> {
    unsafe {
        ::core::ptr::NonNull::new_unchecked(::core::mem::transmute::<
            *mut (dyn $trait<...> $(+ $traits)*),
            *mut Self
        >(from.as_ptr() as _))
    }
}
```

Here `from.as_ptr()` is a thin `*mut TConcrete`, `as _` coerces it to
`*mut (dyn Trait + markers)` (creating the fat pointer via unsized coercion),
and then `transmute` re-casts it to `*mut Self` where `Self = dyn Trait +
markers`.  Since the source and destination are the *same type*, the `transmute`
is a no-op type coercion that the compiler should be able to eliminate entirely.

## Example (dynamic impl – lifetime change)

```rust
fn shroud(from: NonNull<dyn Trait + 'in>) -> NonNull<dyn Trait + 'out> {
    unsafe {
        NonNull::new_unchecked(transmute::<
            *mut (dyn Trait + 'in),
            *mut (dyn Trait + 'out)
        >(from.as_ptr() as _))
    }
}
```

This changes the lifetime annotation of a fat pointer.  In the Rust memory
model, the data pointer and vtable pointer of a trait object do not change with
the lifetime annotation; only the type-system-level bound changes.  A pointer
cast of the form `ptr as *mut (dyn Trait + 'out)` is not directly expressible
with `as` syntax today (lifetime-only coercions are not first-class casts in
stable Rust), so `transmute` is currently the only option on stable.  However,
the code should be clearly commented to explain this limitation.

## Why Clippy Warns

Running `cargo clippy -- -W clippy::pedantic` produces this warning for each
transmute:

```
warning: transmute from a pointer to a pointer
  --> phylactery/src/shroud.rs:…
   |
   | transmute::<*mut (dyn …), *mut Self>(…)
   |
   = help: consider using `… as *mut Self`
```

## Two Distinct Cases

### Case 1: Non-dynamic impl (blanket impl, concrete → dyn)

`from.as_ptr() as _` already performs the correct unsized coercion from
`*mut TConcrete` to `*mut (dyn Trait + markers)`.  The subsequent `transmute`
to `*mut Self` is a no-op because `Self = dyn Trait + markers`.

**Fix:** Replace the `transmute` with a plain `as *mut Self` cast, or simply
drop the `transmute` and use the coerced pointer directly.  However, because
the macro generates code where `Self` is a type parameter, a direct `as` cast
may not be syntactically allowed.  An explicit `as *mut (dyn Trait<…> + traits
+ 'lifetime)` in the macro expansion could replace `transmute`.

### Case 2: Dynamic impl (dyn + 'in → dyn + 'out, lifetime extension)

There is **no stable Rust syntax** to cast `*mut (dyn T + 'a)` to `*mut (dyn
T + 'b)` with `as`.  The transmute is the only way to express this today.

**Fix:** Keep the `transmute` but:
1. Add a `#[allow(clippy::transmute_ptr_to_ptr)]` or `ptr::from_raw_parts` when
   it becomes stable.
2. Add a prominent safety comment explaining *why* transmute is necessary and
   asserting that the representation of `dyn T + 'a` and `dyn T + 'b` are
   identical (both are fat pointers `(data_ptr: *const (), vtable: &VTable)`).

## Soundness Assessment

The transmutes are **sound** given the current library design:
- Fat pointer layout (`*mut dyn Trait + 'short` vs `*mut dyn Trait + 'long`)
  is identical in memory — no layout difference exists for different lifetimes.
- The Soul's `Drop` impl guarantees the data remains valid for the extended
  lifetime.

The issue is one of code clarity, lint compliance, and future-proofing, not
a correctness bug.

## Plan to Fix

1. For **Case 1** (non-dynamic, `TConcrete → dyn Trait`): wherever the
   transmute source and destination are the same concrete type, replace with a
   direct coercion.  Audit each macro arm.

2. For **Case 2** (dynamic, lifetime change): add `#[allow(clippy::transmute_ptr_to_ptr)]`
   with a `// REASON:` annotation that explains the transmute is required
   because stable Rust provides no `as`-cast syntax for lifetime-only changes
   on fat pointers.

3. Extract the safety comment into a shared doc template to keep all `shroud`
   impls consistent.

4. File a tracking note (in the code or in a `TODO` comment) to revisit when
   `std::ptr::from_raw_parts` / `ptr::metadata` stabilises fully, at which
   point fat-pointer lifetime transmutes can be replaced with:
   ```rust
   let meta = ptr::metadata(from.as_ptr());
   NonNull::from_raw_parts(from.cast(), meta)
   ```
