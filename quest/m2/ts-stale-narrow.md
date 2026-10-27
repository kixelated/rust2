# [M] Tests TypeScript narrowed stale

## Goal

The TypeScript translation passes `tsc` where TypeScript keeps a narrowing the program
no longer holds or the fold can't see: a match after a call that changes the place, as
`match self.state { Dot => { self.step(); match self.state { Line(..) => .. } } }`, and
a match after a library call's test, as `if order == Ordering::Less { match order { .. } }`.
The emitted JS doesn't change, and differential tests against native Rust that
type-check prove it.

## Plan

Found by [#110](https://github.com/kixelated/rust2/pull/110) on 2026-10-27; both fail
before it too. TypeScript keeps a property's narrowing across a call, and `===` narrows
as a switch does, so the test needs a type-only widening, and its payload reads a cast.
Prefer one mechanism beside `decided.rs` over casts per call site.
