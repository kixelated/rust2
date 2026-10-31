# [S] Methods of std's ranges

## Goal

[rust2-ts](/ts/README.md) translates every stable method of std's six range types, for
every integer width, `NonZero` of an unsigned one, and `char`, and `contains` of a float
range: `contains`, `is_empty`, `==`, `start`, `end`, `into_inner`, `Debug`, `Hash`, and
`RangeBounds`'s `start_bound`, `end_bound`, and `contains`, proven by differential tests
against native Rust. No range that translates throws `stepping a range of no integer type`
at runtime.

## Plan

Gaps [#320](https://github.com/moq-dev/rust2/pull/320) left. Planned without a moq
consumer, under the maintainer's 2026-10-27 decision that rust2 aims to support all of std.

- Compare bounds as std's `RangeBounds::contains` does, by `<=` and `<`, so a NaN bound or
  item contains nothing, and an exhausted inclusive range is empty and unequal to a fresh one.
- `==` and `Hash` of a range are its fields', `exhausted` included, through the runtime's
  functions of what a key holds.
- A range of `NonZero`s steps as std's does, saturating where its integer's wraps.
- `clone` belongs to [Cloning std's iterators](/quest/m3/ts-iter-clone.md).

Public API: the translator's output.
