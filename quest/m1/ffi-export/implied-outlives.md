# [S] Owned implementors for a parameter that must outlive the call

## Goal

A generic parameter whose bounds imply a lifetime, such as `impl Kept` with
`trait Kept: 'static`, takes an implementor by value, never a borrow of what C lends for
the call. Today [`each`](/ffi/src/collect.rs) reads only the item's direct outlives bounds,
so it picks `&Point` for `keep(kept: impl Kept)` when `Kept` is implemented for both `Point`
and `&'static Point`, and the generated glue fails to compile
(`E0597: a0 does not live long enough`): the whole export fails to build instead of
exporting `keep` at `Point`.

## Plan

Found by [#197](https://github.com/kixelated/rust2/pull/197) on 2026-10-29, which made
`choose` read lifetimes from the elaborated bounds and left `each` reading direct ones.

- It is not unsound: the glue borrows a handle or record through a guard local to the call
  (`rt::Object::borrow`, then `&*local`), so rustc rejects any borrow that must outlive it.
  The cost is a build failure in generated code, with no span in the user's crate, where
  the rules say an unsupported construct fails loud at its source.
- Reproduced on 2026-10-29 by adding `Kept` (with a by-value method, so it is no callback,
  and excluded as the fixture's `Payload` is) and `keep` to the api fixture.
- Read the outlives bound one way for both `choose` and `each`, from the elaborated
  clauses, so the inference rules stay in one place.
- No consumer hits it yet: moq's bounds that imply `'static` reach `choose`, not `each`.

Public API: none, beyond exports that now build.
