# [S] Boxed slices

## Goal

[rust2-ts](/ts/README.md) translates `Box<[T]>`: made by `From`, `collect`,
and `Vec::into_boxed_slice`, read through `Deref`, and turned back by `into_vec`, of bytes
and of other values, proven by differential tests against native Rust.

## Plan

Found writing the `concat` fixture: `Box::<[u8]>::from(&[u8])` and `collect` into a
`Box<[u16]>` refuse as unsupported calls. A `Box<T>` is its value, so a boxed slice is
likely the slice's form; `concat` and `join` then read its items as they read a `&[T]`'s.
