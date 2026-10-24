# [L] Translate moq-net's model to TypeScript

## Goal

[rust2-ts](/ts/README.md) translates moq-net's model layer and the kio primitives it
shares state through, as it translates the codec today, so moq can compare the
source-translator route against its WASM build on the code a browser session runs.

## Plan

Decided with the maintainer on 2026-10-23: moq still ships WASM first; this is the
research track. Once the model translates, moq compares it on the
[TS benchmark](/quest/m2/ts-bench.md), consuming the driver as the `ts` package of
this flake.

The codec's differential harness is the pattern to repeat. The shared npm runtime
(for checked integer ops and the rest of `rt.ts`) is published only once two
generated packages need it.

Decided with the maintainer on 2026-10-23, for the subset:

- Out for good: `unsafe` and raw pointers (all in `model/frame.rs`'s `Segment`),
  and trait objects except at boundaries (`Box<dyn Future>`, transport erasure).
  Those bind to hand-written TS through config instead of translating.
- `u128` stays, on the [wide-integer](/quest/m2/ts-wide-ints.md) representation.
- What the census (`just ts census`, nightly) still ranks after the required quests
  (`Bytes`, `Duration`/`Instant`, cross-crate generics) lands here or splits into
  its own quest.

Public API: the translator's output for moq-net's model.

## Required

- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - kio closes a channel when its last producer drops
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - kio's producers and consumers are futures
- [Shared ownership](/quest/m2/ts-shared.md) - kio's waiters are `Arc`/`Weak` with observable counts
- [Iterators](/quest/m2/ts-iter.md) - the model's adaptor chains
- [Collections](/quest/m2/ts-collections.md) - the model's maps and queues
- [Formatting](/quest/m2/ts-format.md) - panic and error messages match native
- [Tail loops](/quest/m2/ts-tail-loop.md) - 62 moq-net sites end a function in a `loop`
