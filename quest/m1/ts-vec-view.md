# [S] Vec views of the right type

## Goal

[rust2-ts](/ts/README.md)'s `vec_view` (`ts/src/lower/library.rs`) decides whether a `Vec`
is a byte vector from the `Vec`'s own type, so `Vec::as_slice` no longer panics and a
`Vec<Foo<u8>>` is never translated as bytes, proven by differential tests against native
Rust. `just ts census` passes at the moq revision
[the exporter's pin bump](https://github.com/kixelated/rust2/pull/144) moves to.

## Plan

Found by [#144](https://github.com/kixelated/rust2/pull/144) on 2026-10-27: the `as_slice`
case passes the element type where the `Vec` type is expected. A silent miscompile on any
pin, so reproduce it on `main` first, then find every caller that mixes the two.

Public API: the translator's output.
