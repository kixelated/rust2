# [M] Byte slices in locals

## Goal

rust2-ts keeps a byte slice local, and a tuple, `Option`, `Result`, or `ControlFlow` of byte
slices and scalars, as separate JS values, so std's splits, `ok_or`, and `?` on it make no
object. Chrome's varint and frame decode on `just ts bench` are faster than A/A's spread,
and every differential test still passes.

## Plan

The [epic's](/quest/m2/ts-byte-views/README.md) hand edit of locals alone measured Chrome
1.40x on varint decode and 1.30x on frames, Firefox 1.8x, Node and Bun 1.4-1.6x.

- A MIR local qualifies when it has no `&mut` borrow, whole or in part, and every
  definition is a known producer (`split_first`, `split_first_chunk`,
  `split_at(_checked)`, `Option::ok_or`, `Try::branch`), an aggregate, or a copy of
  another qualifying local. A shared borrow materializes the slice, which is safe while it
  is borrowed.
- Any other use (an unknown call, a field store, an aggregate that does not qualify)
  materializes the object at that use. A local that would materialize more than once on a
  path, or inside a loop, stays an object, so nothing allocates more than today.
- An error part stays one opaque value; a local with a `Drop` terminator drops its error
  part when the variant is `Err`, or is left out of the first cut.
- Build `ok_or`'s error only on the `Err` path when it is a pure aggregate.
- Leave out locals saved across an `.await`.

Public API: the translator's output for byte slices.
