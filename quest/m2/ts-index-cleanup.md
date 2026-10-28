# [S] Replacing an indexed element whose drop may panic

## Goal

rust2-ts translates `v[i] = x`, where dropping the old element may panic, into
TypeScript that `tsc` accepts and that drops and replaces as native Rust does on every
path, a panic in the old element's `Drop` included, proven by differential tests.

## Plan

Found while testing [trait objects](/quest/m3/ts-dyn.md). The drop-and-replace through
`IndexMut` lowers the borrowed element as `v[at2]`, where `at2` is a `const` the body
declares inside the `try`. The unwind cleanup in the `catch` replays the assignment as
`v[at2] = x`, out of `at2`'s scope, so `tsc` rejects the output. A `Vec` of a type with
a `Drop` impl reproduces it, no trait object needed:

```rust
let mut v = vec![Bomb::new()];
v[0] = Bomb::new();
```

kio's `self.children[index] = Some(Occupant { .. })` and `waiters[i] = Waiter::noop()`
have that shape.

- Fix it where the cleanup reads what the body spilled, so every temporary an aliased
  place names is in scope in the `catch`, not by special-casing indexing.

Public API: none.
