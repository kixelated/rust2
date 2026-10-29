# [M] A `BytesMut`'s capacity

## Goal

[rust2-ts](/ts/README.md) translates `BytesMut::capacity`, `split_off`, `try_reclaim`,
`try_unsplit`, and `spare_capacity_mut` with `bytes`' values, proven by differential tests
against native Rust.

## Plan

Found by [#222](https://github.com/kixelated/rust2/pull/222) on 2026-10-29: `bytes`
reclaims a `BytesMut`'s storage in `reserve` once no other handle of it is alive, so its
capacity depends on whether the `Bytes` it froze and the handles it split off dropped.
`rt.ByteSlice`, which a `Bytes` is, is a value JS copies and counts no drops, so #222
refuses these. moq calls none of them.

- Count the live handles of a storage, as [shared ownership](/quest/m2/ts-shared.md) counts
  an `Arc`'s, which makes a `Bytes` a value that drops.
- Measure the count's cost on the codec's [TS benchmark](/ts/README.md#bench).

Public API: the translator's output.
