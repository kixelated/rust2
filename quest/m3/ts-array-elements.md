# [S] Arrays a collection holds through generic code

## Goal

rust2-ts translates a `HashMap`, `HashSet`, `BTreeMap`, or `BTreeSet` of `[T; N]` keys
that `tsc` accepts, and a collect or extend of `[u8; N]` items in a function generic over
them, proven by differential tests against native Rust.

## Plan

Found by `hints.rs` in the lang fixture: a collection of `[u32; 275]` keys casts its
`rt.cmpSlice` collection of `ArrayLike` keys to one of `u32[]`, which `tsc` rejects, and
a generic `fill` of `[u8; 3]` items refuses as "`Iterator::collect` of a reference to an
array of bytes it is generic over". [ts-byte-key-refs](/quest/m3/ts-byte-key-refs.md)
lends such keys out; this one stores them.

Public API: the translator's output.
