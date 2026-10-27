# [S] Small std gaps

## Goal

rust2-ts translates `bool::then`, `bool::then_some`, `iter::successors`, and
`usize::from(bool)`, and `None.ok_or(e)` passes `tsc` where TypeScript narrowed the
value to `undefined`, so `discovery.rs`'s parent walk translates, proven by differential
tests against native Rust.

## Plan

Found by [#87](https://github.com/kixelated/rust2/pull/87) and
[#106](https://github.com/kixelated/rust2/pull/106) on 2026-10-27.

- `then` calls its closure only when true, and `successors` calls its closure only for
  the next item it yields; cover both laziness points in the differential tests.
- Report moq-net's census before and after.

Public API: the translator's output.
