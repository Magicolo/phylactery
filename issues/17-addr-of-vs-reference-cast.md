# Issue 17: `value_ptr` and `count_ptr` Use `&self.x as *const _ as _` Instead of `addr_of!`

## Summary

In `Soul::value_ptr` and `Soul::count_ptr`, raw pointers to struct fields are
obtained by creating a shared reference (`&self.field`) and then casting it
through `*const T` to `*mut T`.  The Rust Nomicon and Miri's Stacked Borrows
model recommend using `core::ptr::addr_of!` (or `core::ptr::addr_of_mut!`) to
create raw pointers to fields without forming intermediate references.
The current pattern, while practical today, creates an unnecessary shared
reference to a field that is subsequently treated as a mutable raw pointer.

## Location

`phylactery/src/soul.rs`, lines 149-161.

```rust
fn value_ptr(self: Pin<&Self>) -> NonNull<T> {
    // &self.value as *const _ as _  =>  *const T as *mut T
    unsafe { NonNull::new_unchecked(&self.value as *const _ as _) }
}

fn count_ptr(self: Pin<&Self>) -> NonNull<AtomicU32> {
    unsafe { NonNull::new_unchecked(&self.count as *const _ as _) }
}
```

## Why This Is an Issue

### The cast `&self.field as *const _ as *mut _` is a latent hazard

Under the Rust memory model and Miri's Stacked Borrows model:

1. `&self.value` forms a **shared reference** (`&T`), which carries an implicit
   "SharedReadOnly" (or "Frozen") provenance.
2. `*const T` preserves that provenance.
3. Casting `*const T` to `*mut T` converts to a "mutable" raw pointer type,
   but the provenance remains read-only (derived from a `&T`).

Using the resulting `*mut T` for reading is sound (the current code only calls
`NonNull::as_ref()`, which produces `&T`).  But the provenance mismatch
(`*mut T` derived from `&T`) is a code smell that may:

- Violate Miri's Stacked Borrows under certain interleaving scenarios.
- Be flagged by future stricter versions of the Rust memory model.
- Confuse future maintainers who might believe they can write through the
  pointer.

### The correct idiom: `addr_of!`

`core::ptr::addr_of!(self.value)` creates a `*const T` with raw provenance,
**without forming a reference**.  This avoids the aliasing model concern
entirely.  For a `Pin<&Self>`, the field is behind a shared reference, so we
should use `addr_of!` (not `addr_of_mut!`):

```rust
use core::ptr::addr_of;

fn value_ptr(self: Pin<&Self>) -> NonNull<T> {
    // Safety: `Soul` is pinned; the returned pointer remains valid as long as
    // the Soul lives, which is guaranteed by the reference-counting in `sever`.
    unsafe { NonNull::new_unchecked(addr_of!((*self.get_ref()).value) as *mut T) }
}

fn count_ptr(self: Pin<&Self>) -> NonNull<AtomicU32> {
    unsafe { NonNull::new_unchecked(addr_of!((*self.get_ref()).count) as *mut AtomicU32) }
}
```

Or equivalently, using pointer arithmetic from `self`:

```rust
fn value_ptr(self: Pin<&Self>) -> NonNull<T> {
    let ptr: *const Soul<T> = &*self as *const Soul<T>;
    // Safety: `value` is at a fixed offset inside the Soul struct.
    unsafe { NonNull::new_unchecked(core::ptr::addr_of!((*ptr).value) as *mut T) }
}
```

Note: `Pin::get_ref` returns `&Self` (not `Pin<&Self>`), then `addr_of!` on
the inner field avoids creating a reference to the field itself.

### Why `addr_of!` is preferred

From the Rust Reference and Nomicon:
> Use `ptr::addr_of!` to create a raw pointer to a field without creating a
> reference to the whole struct or to the field.  This is especially important
> for fields that are not properly aligned or that contain `UnsafeCell`.

`Soul.value` may contain `UnsafeCell` (for interior mutability in `T`).  Using
`&self.value` to form a reference and then converting to a raw pointer could
in theory violate the aliasing rules for `UnsafeCell` fields.  `addr_of!`
avoids this concern.

### Current behaviour in Miri

Miri with the current Stacked Borrows model does not flag the existing code
because reading through the resulting pointer is valid.  However, the Tree
Borrows model (which may replace Stacked Borrows) has different rules around
raw pointers derived from shared references.  Switching to `addr_of!` future-
proofs the code against stricter interpretations.

## Plan to Fix

1. Replace `&self.value as *const _ as _` with `addr_of!(self.value) as *mut T`
   (or via `Pin::get_ref` and `addr_of!` on the deref) in both `value_ptr` and
   `count_ptr`.
2. Verify with `cargo +nightly miri test --all-features` that no regressions
   are introduced.
3. Update the `// Safety:` comments to reference `addr_of!` semantics:
   > "We use `addr_of!` to obtain a raw pointer to the field without creating
   > an intermediate reference, preserving the raw provenance that is required
   > for a pointer that will outlive the current borrow."
