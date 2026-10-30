# [S] A B-tree's drop order

## Goal

[rust2-ts](/ts/README.md) drops a whole `BTreeMap` or `BTreeSet` whose entries drop in the
order std's does, proven by the lang fixture's differential tests against native Rust.

## Plan

Found on 2026-10-30 while testing ranges of `Bound`s: at the pinned nightly, std's
`Drop for BTreeMap` calls `drop_tree` (`btree/navigate.rs`), which drops each node's
entries and then moves on in postorder, so a parent's entries drop after its children's.
The runtime drops them in key order, so a map of 12 or more entries that log as they drop
logs a separator key last natively and in place translated. Std's node size is 11 keys and
the runtime's 64, so the order depends on std's node layout, which insertion order decides:
porting it means modelling std's splits and merges for maps whose entries drop, or
refusing with a span the drop of a map whose entries drop and that may hold more than one
node.

`IntoIter`'s drop, through `dying_next`, stays in key order.

Public API: the translator's output for B-trees of values that drop.
