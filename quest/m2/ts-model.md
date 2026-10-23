# [L] Translate moq-net's model to TypeScript

## Goal

[rust2-ts](/ts/README.md) translates moq-net's model layer and the kio primitives it
shares state through, as it translates the codec today, so moq can compare the
source-translator route against its WASM build on the code a browser session runs.

## Plan

Decided with the maintainer on 2026-10-23: moq still ships WASM first; this is the
research track. Once the model translates, moq compares it, consuming the driver as
the `ts` package of this flake.

The codec's differential harness is the pattern to repeat. The shared npm runtime
(for checked integer ops and the rest of `rt.ts`) is published only once two
generated packages need it.

Public API: the translator's output for moq-net's model.

## Required

- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - kio closes a channel when its last producer drops
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - kio's producers and consumers are futures
