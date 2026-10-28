# [XS] An inclusive range drained by a fold

## Goal

A `RangeInclusive` drained through `by_ref` by a fold (`any`, `find`, `for_each`) ends in
core's state, `exhausted` with `start == end`, so `start()`, `end_bound`, and `Debug`
read what std's do, proven by a differential test against native Rust.

## Plan

Found by [#124](https://github.com/kixelated/rust2/pull/124) on 2026-10-27: the runtime
folds through `next` and leaves `start` past `end`.

Public API: the translator's output.
