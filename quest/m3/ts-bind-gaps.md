# [S] Binding gaps

## Goal

rust2-ts's [bindings](/ts/README.md) cover the cases
[#127](https://github.com/kixelated/rust2/pull/127) refuses: a bound type as a collection
key through its binding's `Hash`, `Eq`, and `Ord`, and a path through a renamed dependency
(`foo = { package = "bar" }`) resolving by the name the package gives it.

## Plan

Found by #127 on 2026-10-27. Each fails with the Rust item's span today.

Public API: `ts.toml`'s bindings.
