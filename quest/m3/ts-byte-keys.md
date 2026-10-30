# [S] Byte arrays compared for equality and hashed

## Goal

rust2-ts translates `==` and `!=` of byte arrays such as `[u8; 2]`, and a `[u8; N]`,
`&[u8]`, or `Vec<u8>` as a hash key or inside a tuple, `Option`, or `Arc` that is one,
matching std's equality and finding each key as std's map does. Proven by differential
tests against native Rust.

## Plan

Each refuses with a span today: `==` of byte arrays as a call over a reference to an array
of bytes, and a key as "comparing" or "hashing `[u8; 2]` as a key". Their order translates
through `rt.cmpBytes`, which reads each byte form, a typed array, an `rt.ByteSlice`, or an
`rt.ByteVec`, at run time ([#259](https://github.com/moq-dev/rust2/pull/259)); equality and
hashing can take the same approach. rust2 aims to support all of std.

Public API: the translator's output.
