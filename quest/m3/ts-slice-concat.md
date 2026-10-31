# [S] concat and join of slices of arrays

## Goal

[rust2-ts](/ts/README.md) translates `concat` and `join` of a slice of arrays or slices,
such as `[[u8; 8]]::concat`, proven by differential tests against native Rust.

## Plan

Found writing a fixture: `[hash_a, hash_b].concat()` of `[u8; 8]`s refuses as an
unsupported call. Bytes are typed arrays and other items JS arrays, so each form joins
its own way.
