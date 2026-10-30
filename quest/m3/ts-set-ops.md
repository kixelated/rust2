# [S] Set operations

## Goal

`union`, `intersection`, `difference`, and `symmetric_difference` of a `HashSet` and a
`BTreeSet` translate, each yielding what std's iterator yields in std's order for a
B-tree, proven by a differential test against native Rust that covers `[u8; N]` elements.

## Plan

Each is an unsupported call today for every element type. A hash set's order is the
runtime's, so the hash forms need only yield the same elements; a B-tree's merge order is
std's. Their items are `&T`, which the collection mappings already lend as a slice where
`T` is a `[u8; N]`.

Public API: the translator's output.
