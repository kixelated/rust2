# [S] Debug and Display of Arc and ranges

## Goal

rust2-ts formats `Arc<T>` as `T` formats with `Debug` and `Display`, and std's ranges with
`Debug`, byte for byte as std does, including format specs a range passes to its bounds.
Proven by differential tests against native Rust.

## Plan

Suggested by [#177](https://github.com/kixelated/rust2/pull/177) (`Arc`, refused for every
`T`) and [#179](https://github.com/kixelated/rust2/pull/179) (ranges) on 2026-10-27. rust2
aims to support all of std. A drained `RangeInclusive` prints `exhausted` in its `Debug`,
which [the range fold](/quest/m3/ts-range-fold.md) makes observable.

Public API: the translator's output.
