# [XS] Tail loops in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates a `loop` that ends a function body, exited by
`return` or `break value`, which it refuses today as "a loop outside a scope",
proven by differential tests against native Rust.

## Plan

Decided with the maintainer on 2026-10-23: its own quest rather than part of the
model quest, since it lands alone. The census counts 62 such sites in moq-net, generic functions included.
The async spike's THIR or MIR choice does not block it: the fix is small either
way.

Public API: the translator's output for tail loops.
