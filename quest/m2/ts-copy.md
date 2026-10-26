# [S] Copy by type

## Goal

Copying a `Copy` aggregate in translated TypeScript copies its values and shares its
references, as Rust does. Today `rt.copy` deep-copies the aggregate, references
included: copying `(&Cell<u8>, u8)` clones the cell, so a write through the copy's
reference changes the clone instead of the original. Differential tests against
native Rust cover tuples, arrays, `Option`, `Result`, and structs holding shared
references to interior-mutable values.

## Plan

Decided with the maintainer on 2026-10-26, from a review finding on
[#78](https://github.com/kixelated/rust2/pull/78). The bug predates #78 and is a
silent miscompile, so either fix it or refuse the shapes it can't copy, with a span.

- Copy by the static type: the lowering knows which fields are references, so it
  can emit a shallow copy for those and a value copy for the rest, instead of
  one generic `rt.copy`.
- Measure the codec bench before and after; a typed copy should be no slower.

Public API: the translator's output.
