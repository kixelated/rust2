# [M] Ordering in std

## Goal

rust2-ts translates `cmp::max` and `cmp::min` (the free functions), the `is_sorted`
family, and `BinaryHeap`, proven by differential tests against native Rust, and the cost
of [#173](https://github.com/moq-dev/rust2/pull/173)'s comparison tests on integers is
measured and, if it shows, removed.

## Plan

Planned on 2026-10-28; rust2 aims to support all of std, so this has no moq consumer.
#173 found `cmp::max` and `cmp::min` refused.

- These compare through `PartialOrd::lt` and the rest as #173 builds them, so a crate's own
  `lt` is called where std calls it. `BinaryHeap`'s sift order decides which of equal
  elements comes out first and which comparisons run, so port std's.
- #173 passes `rt.cmpLt(cmp)` where sorts and `max`/`min` took `cmp`. Measure sorts and
  `max`/`min` of integers on the [TS benchmark](/ts/README.md#bench) against the
  commit before #173.

Public API: the translator's output.
