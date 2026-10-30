# [S] Equality through a held type's own ne

## Goal

`==` of slices, `Vec`s, and `VecDeque`s of a crate's type whose `PartialEq` overrides `ne`,
and `!=` of a tuple or an `Arc` holding one, translate, proven by a differential test
against native Rust.

## Plan

Refused today with "through a held type's own" at the comparison's span: the runtime's
`eqSlice`, `eqTuple`, and `eqArc` compare by `eq`, where std's slices call each item's
`ne`, and a tuple's and an `Arc`'s `ne` call what they hold's. Items of unlike types
already compare by `!ne` through the crate's impl (`item_eq` in the collections
lowering); items of one type could do the same, and a tuple's or `Arc`'s `ne` could
lower by its fields' `ne`.
