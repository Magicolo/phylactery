# Issue: `FnOnce` Shroud Creates Unusable `Lich` Instances

## Summary

The library implements `Shroud` for `dyn FnOnce(…) -> R` (and its `Send`/`Sync`/`Unpin` combinations), allowing users to call `soul.bind::<dyn FnOnce() -> R>()`. However, the resulting `Lich<dyn FnOnce() -> R>` is **completely unusable** — calling a `dyn FnOnce` requires ownership (`self`), but `Lich` only provides shared access (`&T`) through `Deref`. This is a confusing API pitfall.

## Why This Is an Issue

`FnOnce::call_once` takes `self` by value:
```rust
pub trait FnOnce<Args> {
    type Output;
    fn call_once(self, args: Args) -> Self::Output;
}
```

`Lich<T>` implements `Deref<Target = T>`, which provides `&T`. There is no `DerefMut` or any way to get `T` (by value) from a `Lich`. So while this compiles:

```rust
let soul = pin!(Soul::new(|| 42));
let lich = soul.as_ref().bind::<dyn FnOnce() -> i32>(); // compiles!
```

This does NOT compile:
```rust
let result = (*lich)(); // ERROR: cannot call `dyn FnOnce` by value through `*`
```

The library already has a `compile_fail` test documenting this behavior:

```rust
// phylactery/src/lib.rs, line 99-107
fail!(can_not_call_lich_dyn_fnonce, {
    use core::pin::pin;
    use phylactery::Soul;

    let soul = pin!(Soul::new(|| 42u32));
    let lich = soul.as_ref().bind::<dyn FnOnce() -> u32>();
    let _result = (*lich)();
});
```

The fact that a `compile_fail` test exists suggests the author is aware of this limitation. However, the current situation is suboptimal:

1. **Users waste time** discovering they can bind as `dyn FnOnce` but can't use it.
2. **The error appears at the call site**, not at the bind site, making it harder to diagnose.
3. **The intent is unclear** — is this a deliberate feature (for future use) or an oversight?

## Impact

- **Severity**: Low (API design / DX)
- **No soundness issue** — the unusable Lich is just wasteful, not dangerous.
- **Affected users**: Anyone who tries to bind a closure as `dyn FnOnce`.

## Proposed Fix

### Option A: Remove `FnOnce` Shroud implementations (recommended)

Remove the `shroud_fn!(FnOnce(...))` line from `phylactery/src/shroud.rs`:

```rust
// phylactery/src/shroud.rs, line 192-194
shroud_fn!(Fn(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
shroud_fn!(FnMut(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
// Remove this line:
// shroud_fn!(FnOnce(T0, T1, T2, T3, T4, T5, T6, T7) -> T);
```

This would cause `soul.bind::<dyn FnOnce() -> R>()` to fail at compile time with a clear error: `dyn FnOnce() -> R: Shroud<T> is not satisfied`.

**Note**: `FnMut` has the same problem — calling `dyn FnMut` requires `&mut self`, which `Lich`'s `Deref` (giving `&self`) cannot provide. However, `FnMut` is a supertrait of `FnOnce`, and closures that impl `Fn` also impl `FnMut`, so removing `FnMut` might be more disruptive. At minimum, document the limitation.

### Option B: Document the limitation prominently

Keep the implementations but add clear documentation:

```rust
/// **Note**: While `Shroud` is implemented for `dyn FnOnce` and `dyn FnMut`,
/// the resulting `Lich` instances cannot be called directly because `Lich`
/// only provides shared access (`&T`). Use `dyn Fn` instead for callable Liches.
```

### Option C: Keep but add a deprecation warning

Use `#[deprecated]` on the `FnOnce` implementations to guide users toward `dyn Fn`.

## Files to Modify

- `phylactery/src/shroud.rs` (line 194): Remove or document `FnOnce` Shroud.
- `phylactery/src/shroud.rs` (line 193): Consider documenting `FnMut` limitation.
- `phylactery/src/lib.rs`: Update or remove the `can_not_call_lich_dyn_fnonce` compile_fail test if FnOnce Shroud is removed.

## Verification

- If removing: verify that `bind::<dyn FnOnce>()` no longer compiles.
- If documenting: verify documentation renders correctly.
- All other tests must pass.

## Notes

The `FnMut` case is more nuanced. While `dyn FnMut` cannot be called through `&T`, users might bind as `dyn FnMut + Send + Sync` for the auto-trait bounds. If so, documenting the limitation is preferable to removing it.
