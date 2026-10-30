# [S] A slice's parts by inclusive range, checked range, and fixed size

## Goal

rust2-ts translates the parts of a slice that still refuse with a span: indexing by an
inclusive range (`&v[a..=b]`, `&v[..=b]`, and their `&mut`), `get` and `get_mut` of a
range, and `first_chunk`, `last_chunk`, `split_last_chunk`, and the `_mut` forms of those
and `split_first_chunk`, for bytes and other values, with writes through a mutable part
landing in the slice and std's panics and `None`s. Proven by differential tests against
native Rust.

## Plan

Indexing takes `Range`, `RangeTo`, `RangeFrom`, and `RangeFull` apart in `range_bounds`
(`src/lower/mir/call.rs`); an inclusive range falls through to an unsupported call, and
must panic as std's does at `b == usize::MAX` and past the end. `get` maps only an
integer index. A part of bytes is a view already, and a mutable part of other values the
runtime's `sliceView`, as `&mut v[a..b]` lends it.

Public API: the translator's output.
