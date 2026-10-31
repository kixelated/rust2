# [S] The rest of a boxed slice's impls

## Goal

[rust2-ts](/ts/README.md) translates the `Box<[T]>` and `Box<str>` impls still refused,
with std's semantics, proven by differential tests against native Rust.

## Plan

Found translating boxed slices: each of these refuses with its span as "unsupported call":

- `Clone::clone_from` of a `Box<[T]>`, which std clones into in place when the lengths
  match, by each element's `clone_from`, and otherwise replaces whole.
- `AsRef` and `AsMut` of a `Box<T>`, for any `T`.
- `TryFrom<Box<[T]>>` and `TryFrom<Vec<T>>` for `Box<[T; N]>`.
- `From<Box<str>>` for `Box<[u8]>`, and `str::into_boxed_bytes`.

No consumer calls them yet; they are planned here as part of std.
