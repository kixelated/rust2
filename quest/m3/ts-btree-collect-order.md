# [S] A B-tree's collect by std's sort and dedup

## Goal

[rust2-ts](/ts/README.md)'s `collect` and `from` into a `BTreeSet` or `BTreeMap` pick
the keys std's do, even where a key's `PartialEq`, `PartialOrd`, and `Ord` disagree,
proven by differential tests against native Rust.

## Plan

Found ordering composites of references: std's `BTreeSet::from_iter` sorts by `sort`,
which tests `PartialOrd::lt`, `BTreeMap::from_iter` sorts by `Ord::cmp`, and both then
keep the last of each run of keys equal by `==`. The runtime's `btreeCollect` and
`btreeCollectKeys` sort and dedup by `Ord` alone, so a key whose impls disagree keeps
other entries. Check `extend` and `append` the same way.
