# [M] Drop order in std's folds and iterators

## Goal

[rust2-ts](/ts/README.md) translates the values that drop which it still refuses
inside std's iterators and formatting, each dropping where std's does, in std's order,
proven by the task fixture's differential tests against native Rust. None appears in moq's census at the pinned revision; rust2 aims to support
all of std.

## Plan

Moved from the m2 drop quest on 2026-10-27, once moq-net, its model, and kio had no
drop refusal left ([#139](https://github.com/kixelated/rust2/pull/139)). Each is refused
with a span where std's order is one JS's can't follow yet:

- A closure that owns a value that drops given to a std function that takes another
  value that drops by value, such as `for_each` of items that drop, that keeps it
  other than in an adaptor, or that calls it through `FnOnce` other than where
  `discards` in `library.rs` says, as `or_insert_with` does (`called_closures`).
- `into_keys` and `into_values` of a hash map whose other half drops: `fold` drops it
  after the half it yields, `next` before.
- `zip` of two `TrustedRandomAccess` iterators of items that drop.
- A `flatten` whose inner iterator holds items once ended.
- A B-tree's `collect` of entries that drop, sorted by the crate's own `Ord`.
- `DebugList::entries` of items that drop.

Each `rt.Iter` has a `fold` and `rfold` that drop what it holds where std's do, and the
by-value consumers std folds go through it: build on those rather than `rt.drop`.

Watch the cost: measure any change to a hot path on the
[TS benchmark](/ts/README.md#bench).

Public API: the translator's output for values that drop in std's iterators.
