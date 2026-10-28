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
- [f32](/quest/m3/ts-f32.md) - `f32` and `Duration`'s `f32` methods translate as `f64` does
- [Trait objects](/quest/m3/ts-dyn.md) - `dyn Trait` and `Box<dyn Future>` translate, so a consumer neither rewrites nor binds them
- [Waiting: type-check in process with tsrs](/quest/m3/ts-tsrs.md) - rust2-ts checks bindings through tsrs instead of `tsc` once it is a stable library
- [Fold matches across tests that aren't variant switches](/quest/m3/ts-fold-guards.md) - a match an `if` or guard decided drops its dead arms
- [An inclusive range drained by a fold](/quest/m3/ts-range-fold.md) - a folded `RangeInclusive` ends in core's state
- [bool's combinators from core's MIR](/quest/m3/ts-bool-core.md) - `bool::then` lowers from core, retiring `rt.boolThen`
- [Binding gaps](/quest/m3/ts-bind-gaps.md) - bound types as keys, unit returns checked, renamed dependencies
- [Drop order in std's folds and iterators](/quest/m3/ts-drop-order.md) - the values that drop still refused in std's folds, zips, flattens, and collects drop as std's do
