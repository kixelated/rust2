# [S] Enum variant fields evaluate as written

## Goal

[rust2-ts](/ts/README.md) evaluates an enum variant's fields in the order the Rust
source writes them, as it already does a struct's, proven by a differential test whose
field expressions log as they run.

## Plan

Found in the [tree-shaking spike](https://github.com/kixelated/rust2/pull/32): a
variant built with its fields out of declaration order lowers to an object literal in
declaration order, so `Shape::Rect { height: push(log, 1), width: push(log, 2), .. }`
runs `push(log, 2)` first. A value with statements of its own is spilled in order
already; a call is not. Struct literals spill every non-constant field first when
they reorder; variants can do the same. Check the `Range` constructor and positional
struct fields given by index (`S { 1: a, 0: b }`) for the same reordering.

Public API: none.
