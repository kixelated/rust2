# [S] A crate's own comparison operators

## Goal

rust2-ts translates `<`, `<=`, `>`, `>=`, sorts, and `Ord::max`, `min`, and `clamp` on a
type whose `PartialOrd` overrides `lt`, `le`, `gt`, or `ge`, calling the crate's override
where std does, proven by differential tests against native Rust.

## Plan

Found by [#124](https://github.com/kixelated/rust2/pull/124) on 2026-10-27: default sorts
and comparisons now go through `partial_cmp` in std's operand order, and a comparison that
reaches the crate's own `lt`, `le`, `gt`, or `ge` is refused with its span.

Public API: the translator's output.
