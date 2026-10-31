# [XS] Debug of Bound

## Goal

[rust2-ts](/ts/README.md) translates `{:?}` of a `Bound<T>` and a `Bound<&T>`, such as a
range's `start_bound()`, writing `Included(3)`, `Excluded(3)`, or `Unbounded` with the
formatter's options as std's derived `Debug` does, proven by a differential test against
native Rust.

## Plan

Found by [Methods of std's ranges](/quest/m3/ts-range-methods.md), whose fixture encodes
bounds as bytes instead. Planned without a moq consumer, under the maintainer's
2026-10-27 decision that rust2 aims to support all of std.

Public API: the translator's output.
