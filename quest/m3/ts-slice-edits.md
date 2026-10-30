# [S] A slice's edits in place

## Goal

rust2-ts translates a slice's `fill`, `fill_with`, `rotate_left`, and `rotate_right`, and
`copy_within`, `copy_from_slice`, and `clone_from_slice` of other than bytes, matching
std's results and panics. Proven by differential tests against native Rust.

## Plan

Each refuses with a span today as an unsupported call; only bytes' `copy_from_slice` and
`copy_within` map.
A mutable chunk's view takes them as an array does.

Public API: the translator's output.
