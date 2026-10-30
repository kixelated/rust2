# [XS] A VecDeque's Debug

## Goal

[rust2-ts](/ts/README.md) translates `{:?}` of a `VecDeque<T>`, writing the same text as
std's list, proven by a differential test against native Rust.

## Plan

Found by [#303](https://github.com/moq-dev/rust2/pull/303): `format!("{q:?}")` of a
`VecDeque<()>` refuses with `unsupported call to <VecDeque<()> as Debug>::fmt`, for any
`T`. `fmt.rs` maps `Vec`'s and `BinaryHeap`'s `Debug` to `rt.list`; a deque needs its
elements in order from its head, as its iterator reads them.

Public API: the translator's output.
