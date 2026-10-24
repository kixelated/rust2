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
- Which keys are primitive depends on the
  [wide-integer representation](/quest/m2/ts-wide-ints.md): a hi/lo `u64` is
  not one.
- Each collection's iterators (`iter`, `values`, `range`, `drain`) are `rt.Iter`s,
  as `rt/iter.ts` makes std's, so the adaptors over them already translate. moq-net's
  `BTreeMap::range` takes `RangeFrom` and `RangeTo`, which are not values yet.

Public API: the translator's output for collections.

## Required

- [Wide integer spike](/quest/m2/ts-wide-ints.md) - which keys lower to primitives
