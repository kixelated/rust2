# [XS] std gaps kio reaches

## Goal

rust2-ts translates the std that kio reaches and no other quest owns, proven by
differential tests against native Rust: `trailing_zeros` of every integer width and
`NonZero` (`task.rs`).

## Plan

Found by [#167](https://github.com/kixelated/rust2/pull/167) and
[#183](https://github.com/kixelated/rust2/pull/183). kio's `static` atomics belong to
[shared ownership](/quest/m2/ts-shared.md). Kept apart from
[the rest of std](/quest/m2/ts-model/std-rest.md), whose capacity work is a hot-path change
of its own.

- `leading_zeros` already maps per width; `trailing_zeros` can follow it, including the
  wide integers' representation.

Public API: the translator's output.
