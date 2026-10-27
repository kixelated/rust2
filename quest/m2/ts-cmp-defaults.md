# [S] Default comparison methods

## Goal

rust2-ts translates the default `ne`, `lt`, `le`, `gt`, and `ge` that a crate's own
`PartialEq` and `PartialOrd` impls inherit, and `<[u8] as PartialEq>::eq`, so `a != b`
and `a < b` on derived types translate, proven by differential tests against native
Rust.

## Plan

Found by [#98](https://github.com/kixelated/rust2/pull/98) on 2026-10-27: comparisons
through references now reach the referent's impl, which still refuses an inherited
method. moq-net's census refuses about 60 `ne` calls this way, `path::Path`'s among
them.

- Lower `ne` as `!eq` and the ordering methods from `partial_cmp`, as std's defaults do,
  so a `partial_cmp` returning `None` makes all four false.
- Cover a hand-written `partial_cmp` that returns `None`, and a type whose `eq` and
  `partial_cmp` disagree, in the differential tests.
- Report moq-net's census before and after.
- Then `bench.rs`'s `interleave` can order its timestamps with `a < b`, as Rust writes it.

Public API: the translator's output.
