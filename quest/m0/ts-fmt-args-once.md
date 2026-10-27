# [S] Evaluate each format argument once

## Goal

Every `format!`, `write!`, and `panic!` argument the TypeScript translation evaluates
runs once, in Rust's order, as native Rust does: no dead tuple statement from
`format_args!`'s argument tuple, and no call evaluated again where a placeholder reads it.
A differential case with an argument that has a side effect proves it.

## Plan

The lowering reads a field of an aggregate assigned once as the operand it was built
from. A reference operand folds in the temporary it borrows, such as a call's result, so
each read of the field evaluates the call again.
