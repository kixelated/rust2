# [M] Mutable subslices of other values

## Goal

rust2-ts translates a mutable subslice of a slice, array, or `Vec` of other than bytes,
`&mut v[a..b]`, and `split_at_mut`, `split_first_mut`, and `split_last_mut`, with writes
through the subslice landing in what it came from. Proven by differential tests against
native Rust.

## Plan

A borrow refuses with "a mutable borrow of a subslice of `T`, which JS would copy, having
no views" (`src/lower/mir/place.rs`), `index_mut` of a range falls through to the same
gap, and the splits have no mapping. A mutable chunk is already lent as the runtime's
`Proxy` view (`sliceView` in `rt/iter.ts`), which these can share.

Public API: the translator's output.
