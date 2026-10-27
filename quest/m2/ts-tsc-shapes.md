# [S] Output tsc accepts for writes through Result and Poll compares

## Goal

Translated code that `tsc` rejects today type-checks and matches native Rust in
differential tests: writing through `if let Err(e) = &mut res { *e += 1 }`, and
comparing a `Poll` whose state TypeScript can see with `rt.Pending`.

## Plan

Found by [#83](https://github.com/kixelated/rust2/pull/83)'s tests on 2026-10-26.
The first writes the read-only `rt.Result.error`; the second narrows the `Poll` so
the comparison has no overlap. Both fail `tsc`, so they are loud, not silent.

- Fix each in the lowering or the runtime's types, not by casting at the call site.

Public API: the translator's output and `rt`'s types.
