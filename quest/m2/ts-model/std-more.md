# [M] Std gaps the model reaches

## Goal

rust2-ts translates the std ranges, wrappers, and methods moq-net's model and moq-pattern
call: `Bound` and `RangeBounds`, `Reverse`, `NonZero`, wrapping and saturating arithmetic,
`div_ceil`, tuples' comparisons and `Clone`, `Vec`'s and slices' editing and sorting
methods (`retain`, `swap_remove`, `dedup`, `sort_by_key`, `last_mut`), `Clone` and
`PartialEq` of a `Vec` that is not `Copy`, `BTreeMap::split_off`, `mem::take`, `Ord`'s
`min`, `max`, and `clamp`, and `str`'s byte searches, proven by differential tests against
native Rust.

## Plan

Found by the [model's](/quest/m2/ts-model/README.md) census on 2026-10-27: 246 refusals
over 121 items in the model, and 62 in moq-pattern. `capacity` (cache.rs, group.rs) stays
refused unless the maintainer decides otherwise. Report the census before and after.

Public API: the translator's output.
