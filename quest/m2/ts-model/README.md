# Translate moq-net's model to TypeScript

## Goal

[rust2-ts](/ts/README.md) translates moq-net's model layer, with the workspace crates it
calls (kio, moq-clock, moq-pattern), as it translates the codec today, so moq can compare
the source-translator route against its WASM build on the code a browser session runs.

## Plan

Decided with the maintainer on 2026-10-23: moq still ships WASM first; this is the
research track. Once the model translates, moq compares it on the
[TS benchmark](/ts/README.md#bench), consuming the driver as the `ts` package of
this flake. That comparison adds moq's `js/net/bench/frames.ts`, a group stream read
into a group, to the benchmark's workloads, which hold only its sync frame headers.

The codec's differential harness is the pattern to repeat. The shared npm runtime
(for checked integer ops and the rest of `rt.ts`) is published only once two
generated packages need it.

Decided with the maintainer on 2026-10-23, for the subset: `u128` stays, on the
[wide-integer](/ts/README.md#design) representation.

Decided with the maintainer on 2026-10-27, from a census of the model's 22 public types
(origin's, broadcast's, track's, group's, frame's, and bandwidth's producers and consumers,
`cache::Pool`, `listing::Tracks`, `time::Timestamp`): 4,595 refusals over 1,112 items.
2,588 are calls into the workspace crates, and 424 items refuse nothing else.

- This is an epic; each group of refusals is a child below. moq must work, but nothing in
  the translator is specific to moq: dependencies follow general rules, and moq's own
  changes (safe `cfg` twins of its `unsafe`, `dyn Expiry` as an enum, dropping or binding
  `hashlink`, `slab`, and `smallvec`) are one research-tier quest in moq's tree.
- Until `unsafe` and trait objects translate, internal `unsafe` in moq-net and kio gets
  safe `cfg` twins in moq, so differential tests cover it, and a dependency containing
  `unsafe` is bound by the consumer. These are stopgaps: decided with the maintainer on
  2026-10-27, a stock crate translates with no annotations or config, so
  [unsafe code the translator can model](/quest/m3/ts-unsafe.md) and
  [trait objects](/quest/m3/ts-dyn.md) remove them. Only true platform boundaries (clocks,
  `rand`, DEFLATE) stay bound.
- Each child measures this target by the census of `ts/moq/model.toml`, rooted at those
  types with the boundaries bound, which nightly reports beside the whole crate.
- The end-to-end work stays here: the model's roots, its differential harness, and the
  `frames.ts` workload. moq writes the binding files `model.toml` names, in its own tree.

Public API: the translator's output for moq-net's model.

## Required

- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - kio closes a channel when its last producer drops
- [Shared ownership](/quest/m2/ts-shared.md) - kio's waiters are `Arc`/`Weak` with observable counts, and its atomics and locks are plain values
- [Translate safe dependencies with the package](/quest/m2/ts-model/crates.md) - kio, moq-clock, moq-pattern, and rangemap translate as one program with moq-net
- [Std's mutable references to values JS copies](/quest/m2/ts-model/std-places.md) - `as_mut`, `get_or_insert`, `iter_mut`, and the rest give an `rt.Place`
- [The rest of std the model reaches](/quest/m2/ts-model/std-rest.md) - the std calls the census still refuses
- [Well-known crates without config](/quest/m2/ts-model/stock.md) - rust2-ts maps `bytes`, `rand`, and `tracing` itself, so `model.toml` stops binding and ignoring them
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - kio's producers and consumers are futures
