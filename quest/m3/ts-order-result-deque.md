# [S] Orderings of Result and VecDeque

## Goal

[rust2-ts](/ts/README.md) translates `PartialOrd` and `Ord` of a `Result` and of a
`VecDeque`, by what each holds, proven by differential tests against native Rust.

## Plan

Found ordering composites of references: `Ok::<_, u8>(a).partial_cmp(&Ok(b))` refuses as
"ordering `Result<..>`, which is not supported yet", and a `VecDeque`'s `partial_cmp` and
`cmp` refuse as unsupported calls, whatever the item type. Both compare as std's do: a
`Result` by variant, then by what it holds, and a deque as a sequence, by each item's
own impls.
