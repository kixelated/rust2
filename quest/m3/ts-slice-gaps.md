# [S] Slice windows and chunks, Vec drain, and the order of byte arrays

## Goal

rust2-ts translates a slice's `windows` and `chunks`, `Vec::drain` of a range, and `<`,
`<=`, `>`, `>=`, and `cmp` of byte arrays such as `[u8; 2]`, matching std's iteration,
drop order of drained values, and comparison results. Proven by differential tests
against native Rust.

## Plan

Met by [#252](https://github.com/moq-dev/rust2/pull/252) on 2026-10-29: its ordering
fixtures and bench rewrote around each. rust2 aims to support all of std. Each refuses
with a span today; the byte-array comparison refuses because the mapping of a byte array
it is generic over would take the array, not a slice of it.

Public API: the translator's output.
