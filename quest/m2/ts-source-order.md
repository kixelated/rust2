# [XS] Source order in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) emits each module's items in Rust source order,
whatever order it reaches them in, so adding a root changes only the output of
the code it reaches.

## Plan

Decided with the maintainer on 2026-10-23: source order rather than sorted by
name, so the output reads like the Rust. Adding the benchmark's entry point
(PR #7) reordered a method in the codec's `decode.ts` snapshot with no change
in behaviour.

Public API: the order of the translator's output.
