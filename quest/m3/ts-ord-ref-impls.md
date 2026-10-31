# [S] Orderings by a crate's impl on references

## Goal

[rust2-ts](/ts/README.md) orders a std composite holding references, such as a tuple, an
`Option`, or a slice, by each reference's own `PartialOrd`, where the crate implements it
on `&T` rather than on `T`, as std's impls of the composite call it. Proven by
differential tests against native Rust.

## Plan

Found reviewing iterator comparisons: `(a,) < (b,)` with `a, b: &Odd`, where only
`&Odd` implements `PartialOrd`, is refused at its span as "`PartialOrd::partial_cmp`
does not resolve", because `cmp_fn` peels each reference to what it points at and asks
`Odd` for the impl. Peel a reference only where std's impl forwards through it, as
`pair_partial_cmp` already does for an iterator's items, and the same for `Ord`,
`PartialEq` through `eq_fn`, and `Hash` where they peel.

Public API: the translator's output.
