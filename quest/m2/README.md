# m2: research

## Goal

Research work with a named consumer that is not waiting on it yet: the
TypeScript translator's spikes and features, and the UniFFI fork's gated or
speculative work.

## Plan

Decided with the maintainer on 2026-10-23: the translator measures before it
builds. The benchmark and census come first; each spike's report rescopes or
deletes the quests that follow it.

Decided with the maintainer on 2026-10-24: the translator's output ranks correctness,
then speed, then the size of the consumer's final tree-shaken, minified JS; readability
is not a goal, and types must be solid at the API boundary ([design](/ts/README.md#design)).

Decided with the maintainer on 2026-10-26: the translator targets browsers. A native
runtime such as Node or Bun calls Rust through [rust2-ffi](/ffi/README.md), so Node and
Bun matter only as proxies for V8 and JavaScriptCore, and a speed call that splits them
from the browsers goes to the browsers.

The fork's feature quests folded into the [exporter](/quest/m1/ffi-export.md);
[m1](/quest/m1/README.md#plan) records why.

## Required

- [Byte slices without objects](/quest/m2/ts-byte-views.md) - splitting a `&[u8]` allocates nothing, neither a chunk view nor a `ByteSlice`
- [Smaller output from MIR](/quest/m2/ts-mir-size.md) - the codec bundle shrinks back to THIR's gzipped size
- [A tuple read only by its fields is never made](/quest/m2/ts-fmt-tuple.md) - a one-argument `format!` reads its argument, not a one-element tuple
- [An A/A control that measures noise](/quest/m2/ts-bench-aa.md) - `just ts bench`'s A/A measures only noise, in every engine
- [Time Timestamp's checked_add and Ord](/quest/m2/ts-bench-timestamp.md) - the `u128` paths #89 sped up get a Chrome timing
- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - every value that drops in moq-net and kio drops where Rust drops it
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - moq-net's and kio's async code translates with Rust's lazy futures
- [Shared ownership](/quest/m2/ts-shared.md) - `Arc`/`Rc`/`Weak` keep exact counts, and locks and atomics become plain values
- [Share what the crate never changes in place](/quest/m2/ts-shared-read.md) - copies of `Copy` aggregates no one writes into in place go away, recovering #83's decode cost
- [Matches TypeScript has already narrowed](/quest/m2/ts-narrow-test.md) - an inner match on a value an outer match decided passes `tsc`
- [Small std gaps](/quest/m2/ts-std-gaps.md) - `bool::then`, `iter::successors`, `usize::from(bool)`, and `None.ok_or` narrowing
- [Copy-on-write strings](/quest/m2/ts-cow-str.md) - `Cow<str>` translates, finishing `ietf/namespace.rs`
- [Translate moq-net's model to TypeScript](/quest/m2/ts-model.md) - the driver grows from moq-net's codec to its model, for moq to compare against WASM
- [Waiting: Swift 6.4 in nixpkgs-unstable](/quest/m2/uniffi-swift-arm.md) - the nightly UniFFI fixtures move back to arm
- [Waiting: ubrn reads UniFFI 0.32](/quest/m2/ubrn-release.md) - the binding benchmark's browser runner pins a ubrn release, not an open PR
