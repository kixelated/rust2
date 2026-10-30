# [S] Equality of std containers of unlike elements

## Goal

`==` and `next_if_eq` of std containers whose elements are of other types that compare
by their own `PartialEq<U>`, as a `Vec<Left>` against a `&[Right]` does, and of
`VecDeque` against `Vec`, slices, and arrays, translate, proven by a differential test
against native Rust.

## Plan

Refused at their spans today, as the reject fixture's `unlike_elements` and
`peeked_deque` are: the collections mapping compares only containers whose elements
compare alike. Each element pair could lower through `<T as PartialEq<U>>::eq` as
`next_if_eq` of other types does, lending a `[u8; N]` element as its `rt.ByteSlice`, with
`ne` by the elements' own `ne` where a crate overrides it.

Public API: the translator's output.
