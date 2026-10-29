# [S] Edits of byte vectors and slices

## Goal

rust2-ts translates every `Vec` and slice method on `Vec<u8>` and `[u8]` that it translates
for other element types, such as `retain`, `retain_mut`, `dedup`, `insert`, `remove`,
`swap_remove`, and the slice's `last`, `reverse`, `swap`, and sorts, proven by differential
tests against native Rust.

## Plan

Found by [#204](https://github.com/kixelated/rust2/pull/204) on 2026-10-28, whose fixture
hit `retain` on a `Vec<u8>`: the mappings in `library.rs` skip `u8` elements, which are an
`rt.ByteVec` or `rt.ByteSlice` rather than an array, and nothing maps them instead. rust2
aims to support all of std, so this has no moq consumer.

- Find every such skip rather than only the methods named here, and give each the
  `rt.ByteVec` or `rt.ByteSlice` counterpart, with the closure's drop and the element's
  equality as the generic path has them.

Public API: the translator's output.
