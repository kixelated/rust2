# [S] reduce, max_by, min_by, and Peekable's next_if_eq and peek_mut

## Goal

`Iterator::reduce`, `max_by`, and `min_by`, and `Peekable`'s `next_if_eq` and `peek_mut`,
translate, over items whose `None` is `rt.None` too, proven by a differential test against
native Rust.

## Plan

Found while translating `nth`, `last`, `find`, `max`, and `min` of `Option` items on
2026-10-30: each of these is an unsupported call for any item, as `src/lower/iter.rs` maps
none of them. std's `max_by` keeps the last of equal items and `min_by` the first, as `max`
and `min` do, so the runtime's `best` could take a comparator. Each returns the item's
`None` as the runtime's other consumers now take it.

Public API: the translator's output.
