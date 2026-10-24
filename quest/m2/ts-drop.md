# [M] Drop in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates types that implement `Drop`, which it refuses
today: each value's `drop()` runs where Rust drops it, in Rust's drop order, including
on early return, `?`, and panic, proven by differential tests against native Rust.

## Plan

Decided with the maintainer on 2026-10-23: `Drop` becomes an explicit `drop()`,
exposed as `[Symbol.dispose]` for JS callers. kio's producers close their channel
when the last one drops, so moq-net's model needs this first.

THIR does not say where drops happen; MIR does, after drop elaboration. The
[spike](/quest/m2/ts-async-spike.md) decides whether to read drop points from MIR
beside the THIR lowering or move the lowering itself; rewrite this plan to its
report before starting.

Decided with the maintainer on 2026-10-24: the async spike
([#16](https://github.com/kixelated/rust2/pull/16)) merges with drop points from
THIR's scopes, and the [MIR lowering spike](/quest/m2/ts-mir-spike.md) then
decides whether drops move to MIR's elaboration; rewrite this plan to its report
too. Also in scope, found since: a temporary that drops and is held across an
`.await` is owned instead of refused (3 refusal sites), which needs declarations
at scope level.

Public API: the translator's output for `Drop` types.

## Required

- [Async and drop spike](/quest/m2/ts-async-spike.md) - where drop points come from
- [MIR lowering spike](/quest/m2/ts-mir-spike.md) - rewrite to the spike's report before starting
