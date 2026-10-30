# [S] Binding gaps

## Goal

rust2-ts's [bindings](/ts/README.md) cover a bound type as a collection key, through its
binding's `Hash`, `Eq`, and `Ord`, which [#127](https://github.com/moq-dev/rust2/pull/127)
refuses.

## Plan

Found by #127 on 2026-10-27. Keying a collection by a bound type fails with the Rust
item's span today.

Public API: `ts.toml`'s bindings.
