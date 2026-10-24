# [M] Iterators in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates iterator adaptor chains and user
`impl Iterator`, with Rust's laziness and short-circuiting, proven by
differential tests, including iterators whose items are `Option`.

## Plan

Decided with the maintainer on 2026-10-23: std's adaptors live in `rt.ts` as
lazy classes, since the driver only has THIR for the local crate. Their
`next()` returns the item or a unique `DONE` symbol, so an iterator of
`Option<T>` never collapses into the refused nested `Option`. A user
`impl Iterator` translates to the same protocol, and `for` loops drive any
iterator through it.

- `for` accepts only a real iterator. Today it treats the first argument of
  whatever call built the iterator as a slice when the type is `slice::Iter`;
  the formatting PR (#8) fixed the same bug in `DebugList::entries`. Cover it
  with a differential case.

- Measure against ES iterator helpers on the [TS benchmark](/ts/README.md#bench)
  and record the numbers, but keep the `DONE` protocol unless they show it loses.
- Cover what moq-net uses (about 1,100 adaptor sites); the
  census (`just ts census`) names them.

Public API: the translator's output for iterators.
