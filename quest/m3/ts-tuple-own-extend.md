# [S] A crate's own Extend in a tuple

## Goal

[rust2-ts](/ts/README.md) translates `collect`, `extend`, and `unzip` into a tuple that
holds a crate's own `Default + Extend` type beside std's collections, each field seeing
the calls std makes, proven by differential tests against native Rust.

## Plan

Found by [#340](https://github.com/moq-dev/rust2/pull/340), which refuses such a tuple
with its span. Std's tuple `Extend` calls each field's `extend_reserve` and then
`extend_one` per item. Both are unstable, so a stable crate keeps their defaults:
`extend_reserve` does nothing, and `extend_one` calls the crate's `extend` with a
one-item iterator. The translation calls the crate's `extend` that way, once per item, in
field order. No consumer needs it; it is planned here as part of std.

Public API: the translator's output.
