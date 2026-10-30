# [S] std iterators' Debug

## Goal

[rust2-ts](/ts/README.md) translates `{:?}` of std's iterators, writing the same text as
std, proven by differential tests against native Rust.

## Plan

Found auditing std collections' `Debug`: each of these refuses with `unsupported call to
<... as Debug>::fmt`, while the collections themselves translate.

- `slice::Iter`, `vec::IntoIter`, and `binary_heap::Iter` write their remaining items as a
  tuple of one slice: `Iter([1, 2])`.
- `str::Chars` writes `Chars(['a', 'b'])`, and `str::Bytes` its adaptor chain:
  `Bytes(Copied { it: Iter([97]) })`.
- `btree_map`'s `Iter`, `Keys`, and `Values`, `btree_set::Iter`, and `hash_map`'s and
  `hash_set`'s write their remaining entries as a list or map, without a name.
- `vec_deque::Iter` writes the two slices of its ring buffer, `Iter([3], [1, 2])`, as
  `VecDeque::as_slices` returns them. Both need the runtime's buffer to split where
  std's does, by the same growth and head; `as_slices` is refused too.

Public API: the translator's output.
