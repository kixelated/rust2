# [M] Drop in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates types that implement `Drop`, which it refuses
today: each value's `drop()` runs where Rust drops it, in Rust's drop order, including
on early return, `?`, and panic, proven by differential tests against native Rust.

## Plan

Decided with the maintainer on 2026-10-23: `Drop` becomes an explicit `drop()`,
exposed as `[Symbol.dispose]` for JS callers. kio's producers close their channel
when the last one drops, so moq-net's model needs this first.

THIR does not say where drops happen; MIR does, after drop elaboration. rust-js
moved to MIR for this reason; weigh reading drop points from MIR beside the THIR
lowering against moving the lowering itself.

Public API: the translator's output for `Drop` types.
