# [S] Translate char and vec![x]

## Goal

rust2-ts translates `char` values and `vec![x]` on the pinned nightly, proven by
differential tests against native Rust, which unblocks moq-net's 6 refused `char`
switches and 17 refused `vec![x]` sites.

## Plan

Decided with the maintainer on 2026-10-26, from
[#78](https://github.com/kixelated/rust2/pull/78)'s census.

- **`char`:** a number holding the Unicode scalar value, so it switches, compares,
  and orders like Rust's. Conversions keep Rust's rules: `as u8` truncates,
  `char::from_u32` rejects surrogates and values past `0x10FFFF`, and formatting
  writes UTF-8 or UTF-16 as the target needs.
- **`vec![x]`:** lower the nightly's expansion (`from_elem` or the boxed-slice
  route), including `vec![x; n]` with `Clone` semantics for each element.

Public API: the translator's output.
