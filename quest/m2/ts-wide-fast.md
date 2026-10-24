# [S] Wide-integer fast paths

## Goal

Two fast paths for [rust2-ts](/ts/README.md)'s 64-bit integers (numbers while
safe, limbs past 2^53, per its [design](/ts/README.md#design)), each kept only if
the [TS benchmark](/ts/README.md#bench) measures it faster, with the wide
differential sweep still passing.

## Plan

From [#26](https://github.com/kixelated/rust2/pull/26)'s follow-ups:

- A fused `from_halves`: Chrome's 8-byte varint decode is 25-45% slower than
  `bigint`'s, since it goes through `rt.u64.or(rt.u64.shl(hi, 32), lo)`.
- A product of two numbers past 2^53 boxes both factors into limbs before
  multiplying; multiplying their halves directly would cut the allocations
  `rescale` measures.

`just ts bench --route` times each against main's translation in one run.

Public API: none; the runtime's internals.
