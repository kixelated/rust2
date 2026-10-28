# [S] std gaps kio reaches

## Goal

rust2-ts translates the std that kio reaches and no other quest owns, proven by
differential tests against native Rust: `array::from_fn`, `Default` of arrays (kio's
`[Atomic<u64>; 8]::default()`) and of `()`, and a `HashMap` whose values are `Option`s
(`shared.rs`).

## Plan

Found by [#167](https://github.com/kixelated/rust2/pull/167) and
[#183](https://github.com/kixelated/rust2/pull/183). kio's `static` atomics belong to
[shared ownership](/quest/m2/ts-shared.md). Kept apart from
[the rest of std](/quest/m2/ts-model/std-rest.md), whose capacity work is a hot-path change
of its own.

- `Default` and `from_fn` of an array run element by element in index order, which an
  element whose constructor has effects or panics observes.
- A map of `Option`s needs its `Option`-returning methods (`get`, `insert`, `remove`) to
  tell `None` from `Some(None)`; [#134](https://github.com/kixelated/rust2/pull/134) gave
  `rt.None` for that.

Public API: the translator's output.
