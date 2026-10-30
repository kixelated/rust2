# [S] A B-tree's drop order

## Goal

[rust2-ts](/ts/README.md) drops a whole `BTreeMap` or `BTreeSet` whose entries drop in the
order std's does, proven by the lang fixture's differential tests against native Rust.

## Plan

Found on 2026-10-30 while testing ranges of `Bound`s: at the pinned nightly, std's
`Drop for BTreeMap` calls `drop_tree` (`btree/navigate.rs`), which drops each node's
entries and then moves on in postorder, so a parent's entries drop after its children's.
The runtime drops them in key order, so a map of 12 or more entries that log as they drop
logs a separator key last natively and in place translated. `clear` drops the same way,
and a map of 11 entries can already span three nodes after a removal. Std's node size is 11 keys and
the runtime's 64, so the order depends on std's node layout, which insertion order decides:
porting it means modelling std's splits and merges for maps whose entries drop, or
refusing with a span the drop of a map whose entries drop and that may hold more than one
node.

`IntoIter`'s drop, through `dying_next`, stays in key order.

Measured on 2026-10-30, at moq's pin: of the census's whole-B-tree drops of entries that
drop, those whose drop runs a `Drop` impl, and so can show the order, are 4 in the model
(`auth::Attached`, `bandwidth::Entry`, `group::Handle`, `track::Slot`) and 5 in moq-net
(also `HeldFetch`); the other 12 only count `Arc`s or `Weak`s down. A prototype of std's
layout in TypeScript ran insert, get, iterate, and remove 1.5x (65536 keys) to 2x (100
keys) slower than the runtime's B+ tree. Whether to port it, as the only B-tree or for maps
whose entries run a `Drop` impl, or to refuse those, is the maintainer's, beside the same
question for [hashbrown](/quest/m2/ts-model/hashbrown.md).

Public API: the translator's output for B-trees of values that drop.
