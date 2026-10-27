# [S] str slicing and &str patterns

## Goal

rust2-ts translates `&s[a..b]` on a `str` and the `str` methods that take a `&str`
pattern, so moq-net's `path/mod.rs`, `discovery.rs`, and `lite/compress.rs` translate,
proven by differential tests against native Rust.

## Plan

Found by [#96](https://github.com/kixelated/rust2/pull/96) on 2026-10-27, which
translated `char` patterns and returns UTF-8 byte offsets from `rfind` and
`match_indices`. Slicing is what lets those offsets be used.

- Slice by UTF-8 byte offsets and panic, as std does, on a boundary inside a char.
- An empty `&str` pattern matches between every char in `split`, `trim_*`, and
  `match_indices`; cover it and multi-byte patterns in the differential tests.
- Report moq-net's census before and after.

Public API: the translator's output.
