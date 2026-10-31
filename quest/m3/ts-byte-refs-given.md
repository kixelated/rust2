# [M] References to byte arrays std is given

## Goal

[rust2-ts](/ts/README.md) translates the calls still refused as "of a reference to an
array of bytes it is generic over": std given a `&[u8; N]` where its generic code takes a
`&T` of a `T = [u8; N]`, as `Option<&[u8; N]>::copied` and `cloned`, `mem::swap`,
`replace`, and `take` through a `&mut [u8; N]`, and `slice::contains` of `[u8; N]`
elements, proven by differential tests against native Rust.

## Plan

The reject fixture's `copied_bytes` shows one. `lend::plan` refuses such an input outside
a collection's method, whose runtime reads either form, since each mapping takes the
`Uint8Array` std holds where translated code passes an `rt.ByteSlice`. An
`rt.byteSliceView` of the slice shares its bytes, so it stands in wherever the mapping
reads, copies, or writes through it; check each mapping before admitting it.

Public API: the translator's output.
