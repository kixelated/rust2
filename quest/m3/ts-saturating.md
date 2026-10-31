# [S] Saturating

## Goal

[rust2-ts](/ts/README.md) translates `std::num::Saturating<T>` with std's impls: its
operators, which saturate at the type's bounds, its `Debug` and `Display`, and its
`PartialEq`, `PartialOrd`, `Ord`, and `Hash`, which are its value's, proven by
differential tests against native Rust.

## Plan

Found auditing std types' orderings: `Saturating(a).cmp(&Saturating(b))` refuses as
"constructing `std::num::Saturating<u8>` from another crate", before its `cmp` is reached.
`Wrapping` already translates as the value it holds, with std's impls; `Saturating` can
follow it. No consumer calls it yet; it is planned here as part of std.

Public API: the translator's output.
