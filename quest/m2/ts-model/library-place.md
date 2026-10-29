# [S] A write through a mutable reference to a library object

## Goal

`*r = v`, where `r` is a `&mut` to a library object such as an `Arc` taken out of an
`Option` (`if let Some(r) = v.get_mut(0) { *r = Arc::new(1) }`), writes the slot `r`
points at, proven by a differential test against native Rust.

## Plan

Found by [#201](https://github.com/kixelated/rust2/pull/201) on 2026-10-28, in safe code:
the translation assigns the local the `Option` was matched into, so the `Vec` keeps its old
`Arc`, whose count stays up. It silently differs from native, so it is fixed or refused
before anything else here.

- A `&mut` to a library object is the object, so a whole write through it must go through
  the `rt.Place` of where it is kept, as a `&mut` to a value JS copies does.

Public API: the translator's output.
