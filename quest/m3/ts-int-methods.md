# [M] Integers' inherent methods

## Goal

[rust2-ts](/ts/README.md) translates the inherent methods of every integer width and of
`NonZero`, `Wrapping`, and `Saturating` of one with std's semantics, proven by differential
tests against native Rust at both bounds of each width.

## Plan

Found auditing `Saturating`: each of these refuses as "unsupported call", with its span:
`pow`, `abs`, `signum`, and the `checked_`, `wrapping_`, `saturating_`, and `overflowing_`
forms the runtime lacks, such as `wrapping_neg`, `saturating_pow`, and `saturating_abs`;
`NonZero`'s `saturating_pow` and `saturating_abs`; and `Wrapping`'s and `Saturating`'s own
`pow`, `abs`, `signum`, `count_ones`, `rotate_left`, `swap_bytes`, and the rest, which are
their integer's. The table of integer methods in `ts/src/lower/library.rs` is where each
goes; a wrapper's method is its integer's on the value it holds. No consumer calls them
yet; they are planned here as part of std.

Public API: the translator's output.
