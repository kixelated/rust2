# [XS] Operands evaluate in order around indexing

## Goal

[rust2-ts](/ts/README.md)'s THIR lowering evaluates the value being indexed, or the
slice `copy_within` reads, before an index or `dest` that needs statements of its own,
as Rust does: `f()[{ ... }]` runs `f()` first. Proven differentially under both
lowerings.

## Plan

Found in [#49](https://github.com/kixelated/rust2/pull/49). Each is an operand lowered
to an expression, then a later operand's statements emitted before that expression is
read. `range_bounds` already saves the sliced value first, and `Body::declared` and
`Body::operands` hold the pattern for fields and arguments. Look for other ad-hoc
sequences of the same shape and fix them together. MIR's operands are already locals,
so it may need no change: prove it with the same case.

Public API: none.
