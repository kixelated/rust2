# [XS] Ranges of Bounds

## Goal

rust2-ts translates `Vec::drain`, `VecDeque::drain`, and a B-tree's `range` and
`range_mut` of a `(Bound<T>, Bound<T>)`, an excluded start included, matching std's
items and panics. Proven by differential tests against native Rust.

## Plan

Each refuses with a span since [#259](https://github.com/moq-dev/rust2/pull/259): the
runtime's `bounds` reads std's range types alone, and read a tuple as unbounded. Reading
the tuple's `rt.RangeBound`s there, and an excluded start where `slice::range` and the
B-tree's search take one, covers all four. A crate's own `RangeBounds` stays refused
until it has a consumer. rust2 aims to support all of std.

Public API: the translator's output.
