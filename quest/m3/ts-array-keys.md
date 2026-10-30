# [S] Array keys a collection holds

## Goal

rust2-ts translates a `HashMap`, `HashSet`, `BTreeMap`, or `BTreeSet` of `[T; N]` keys
to TypeScript `tsc` accepts, proven by a differential test against native Rust.

## Plan

Found by `hints.rs` in the lang fixture: a collection of `[u32; 275]` keys casts its
`rt.cmpSlice` collection of `ArrayLike` keys to one of `u32[]`, which `tsc` rejects. Its
`[u8; 3]` items, collected in a function generic over them, refuse as
[ts-byte-refs-taken](/quest/m3/ts-byte-refs-taken.md) describes.

Public API: the translator's output.
