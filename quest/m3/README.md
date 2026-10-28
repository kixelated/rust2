# m3: deferred

## Goal

Work that waits on evidence or has no consumer yet: the build-speed tracks,
each a measurement that decides whether its follow-up lands, and the parts of
std the translators cover before a consumer calls them.

## Plan

Decided with the maintainer on 2026-10-27: rust2 aims to support all of std, so a
generic std gap is planned here even when moq never calls it.

## Required

- [Retrial Cranelift on moq's tests](/quest/m3/cranelift-unwind.md) - waits on upstream: moq's tests pass under Cranelift, so its CI can weigh it again
- [Relink without rebuilding](/quest/m3/relink.md) - measure upstream's skip of dependency-only rebuilds on moq's edits once it reaches nightly
- [Unsafe code the translator can model](/quest/m3/ts-unsafe.md) - a dependency using only `unsafe` the translator can model translates from source
- [The rest of Duration](/quest/m3/ts-duration.md) - compound assignment, float conversions, and `Sum` on a `Duration` translate
- [The rest of format specs](/quest/m3/ts-fmt-specs.md) - precision, argument widths, `+`, and the other radixes format as `core::fmt` does
- [Waiting: type-check in process with tsrs](/quest/m3/ts-tsrs.md) - rust2-ts checks bindings through tsrs instead of `tsc` once it is a stable library
- [Fold matches across tests that aren't variant switches](/quest/m3/ts-fold-guards.md) - a match an `if` or guard decided drops its dead arms
- [A crate's own comparison operators](/quest/m3/ts-partial-ord.md) - an overridden `lt`/`le`/`gt`/`ge` is called where std calls it
- [An inclusive range drained by a fold](/quest/m3/ts-range-fold.md) - a folded `RangeInclusive` ends in core's state
- [bool's combinators from core's MIR](/quest/m3/ts-bool-core.md) - `bool::then` lowers from core, retiring `rt.boolThen`
- [Binding gaps](/quest/m3/ts-bind-gaps.md) - bound types as keys, unit returns checked, renamed dependencies
