# [M] Collections in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates `HashMap`, `HashSet`, `BTreeMap`,
`BTreeSet`, and `VecDeque` with Rust's key semantics (`Hash` and `Eq`, or `Ord`),
proven by differential tests.

## Plan

Decided with the maintainer on 2026-10-23: each collection is one `rt.ts` type
that hides its backing. A hash map whose key lowers to a JS primitive uses a JS
`Map` or `Set`; any other key goes through the translated `Hash` and `Eq`,
since JS compares objects by identity. `BTreeMap` is a sorted structure ordered
by the translated `Ord`, and `VecDeque` a ring buffer.

- `HashMap` iteration order is unspecified in Rust, so insertion order is a
  valid refinement; differential tests must not compare it.
- A 64- or 128-bit integer is a number while it is a safe integer, and an object
  of 32-bit limbs past that ([representation](/ts/README.md#design)), so it is a
  primitive key only while it is a number.
- Each collection's iterators (`iter`, `values`, `range`, `drain`) are `rt.Iter`s,
  as `rt/iter.ts` makes std's, so the adaptors over them already translate. moq-net's
  `BTreeMap::range` takes `RangeFrom` and `RangeTo`, which are not values yet.

Public API: the translator's output for collections.

