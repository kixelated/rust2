# [S] Binding gaps

## Goal

rust2-ts's [bindings](/ts/README.md) cover the cases
[#127](https://github.com/kixelated/rust2/pull/127) refuses or checks loosely: a bound type
as a collection key through its binding's `Hash`, `Eq`, and `Ord`; a binding that returns
unit is checked against `=> undefined` rather than `=> void`, which accepts any return
value; and a path through a renamed dependency (`foo = { package = "bar" }`) resolves by
the name the package gives it.

## Plan

Found by #127 on 2026-10-27. Each fails with the Rust item's span today, apart from the
unit return, which `tsc` accepts unchecked.

Public API: `ts.toml`'s bindings.
