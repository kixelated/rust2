# [S] Iterator consumers over items whose None is rt.None

## Goal

`nth`, `last`, `find`, `max`, `min`, and std's other iterator consumers that return an
`Option` of the item translate over an iterator whose item's `None` is `rt.None`, such as
`Option<u8>`, keeping `None` apart from `Some(None)`, proven by a differential test against
native Rust.

## Plan

Found while fixing a returned `next` on 2026-10-30: `it.nth(n)` and `it.last()` of an
iterator of `Option<u8>` are refused at their span, as `src/lower/library.rs`'s
`takes_none` lists only `next`, `next_back`, and `nth_back` among iterator methods.
Each runtime consumer needs the item's `None` passed in, as `rt.iterToOption` takes it.

Public API: the translator's output.
