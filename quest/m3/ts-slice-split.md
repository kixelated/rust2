# [S] Slices split by a predicate

## Goal

[rust2-ts](/ts/README.md) translates `[T]::split` and its forms by a predicate (`splitn`,
`rsplit`, `split_inclusive`, and their `_mut` and reversed variants), of bytes and of other
values, proven by differential tests against native Rust.

## Plan

Found writing the `concat` fixture: `data.split(|&b| b == 0)` of a `&[u8]` refuses as an
unsupported call, so the fixture splits by hand. Each part of bytes is an `rt.ByteSlice`
of the same data; a `_mut` part of other values is the `Proxy` view `chunks_mut` lends.
