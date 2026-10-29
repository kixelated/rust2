# [S] Into targets through a chain of From implementations

## Goal

`impl Into<X>`, where `X` does not map, takes what converts into `X` through any number of
`From` implementations, not only one. Today [`sources`](/ffi/src/collect.rs) reads the
bounds of the `From` implementations into `X` and takes each `Into` or `AsRef` target
their source parameter implies. When that target does not map either, such as
`impl<B: Into<Inner>> From<B> for Outer` with an unmapped `Inner`, it stops there and the
parameter falls back to `String`, `Bytes`, or `Vec<u8>`, or is refused.

## Plan

Suggested by [#197](https://github.com/kixelated/rust2/pull/197) on 2026-10-29. No consumer
needs it yet: moq's `impl Into<Timed<Bytes>>` is one level. Start once a consumer's
parameter goes through two.

- Follow each target that does not map through its own `From` implementations, carrying
  whether any level must outlive the call, as one level does now. Guard against cycles.
- A fallback that meets the bounds is the bug #197 fixed, so prove the chained case with a
  payload that is not UTF-8, as #197's `timed_hex` does.

Public API: the C, C++, and Kotlin types of parameters that convert through a chain.
