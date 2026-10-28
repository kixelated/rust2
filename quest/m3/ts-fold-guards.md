# [S] Fold matches across tests that aren't variant switches

## Goal

rust2-ts folds a match that an enclosing test already decided when that test is an `if`
or a match guard, as [#110](https://github.com/kixelated/rust2/pull/110) does for a
switch on a variant, so the output drops the dead arms rather than widening them, proven
by differential tests against native Rust.

## Plan

Found by [#118](https://github.com/kixelated/rust2/pull/118) on 2026-10-27: the dataflow
in `ts/src/lower/mir/decided.rs` forgets what it knows at a switch on anything but a
variant, so a match decided before an `if` gets a type-only widening instead of folding.
`tsc` accepts it; this only shrinks the output.

- Fix the doc comments in `ts/src/lower/mir/mod.rs` while there: `whole()` sits between
  `own()`'s doc comment and `own()`.
- Measure the output size on the [TS benchmark](/ts/README.md#bench).

Public API: the translator's output.
