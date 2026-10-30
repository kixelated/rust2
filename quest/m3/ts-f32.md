# [M] f32

## Goal

[rust2-ts](/ts/README.md) translates `f32` as it translates `f64`, including casts between
them, its formatting, and `Duration`'s `f32` methods, proven by differential tests against
native Rust.

## Plan

Decided with the maintainer on 2026-10-27: rust2 aims to support all of std, so this is
planned without a moq consumer. The decisions of `f64` and its formatting, which
[#165](https://github.com/moq-dev/rust2/pull/165) and [#260](https://github.com/moq-dev/rust2/pull/260)
landed, carry over.

- An `f32` is a JS number rounded with `Math.fround` after every operation. A 64- or
  128-bit integer rounds to `f32` once, never through `f64`, which would round twice.
- Formatting prints the shortest digits that round-trip as an `f32`, as `core::fmt` does,
  which JS's number formatting cannot give.

Public API: the translator's output.
