# [S] Matches TypeScript has already narrowed

## Goal

rust2-ts's output passes `tsc` when a value is matched again where an enclosing match
already decided its variant, such as `match x { Dot => match x { Line(..) => .. } }` or a
match on a copy taken inside the arm, proven by differential tests against native Rust
that also type-check.

## Plan

Found by [#100](https://github.com/kixelated/rust2/pull/100) on 2026-10-27, which widens
a local TypeScript narrows by construction but not one it narrows by a test. The inner
test fails `tsc` (TS2367) rather than miscompiling.

- Fold away a test an enclosing test already decided, so the inner match lowers to its
  one reachable arm.
- Where folding can't reach, such as a copy matched in the arm, widen the copy as #100
  widens a constructed local.

Public API: the translator's output.
