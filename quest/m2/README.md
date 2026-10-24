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

## Required

- [MIR lowering spike](/quest/m2/ts-mir-spike.md) - the whole lowering ported to MIR on two fixtures decides whether the translator moves
- [Fail the TS benchmark when Chrome hangs](/quest/m2/ts-bench-chrome-timeout.md) - a hung renderer fails the run loudly
- [Stable names](/quest/m2/ts-stable-names.md) - generic instances and trait-impl methods always carry their suffix, so callers' names never shift
- [Diverging calls through closures and generics](/quest/m2/ts-never-edges.md) - closures returning `!` and generics at `!` pass `tsc`
- [Cheaper byte slices](/quest/m2/ts-byte-views.md) - splitting a `&[u8]` stops making a `Uint8Array` view per split
- [Wide-integer fast paths](/quest/m2/ts-wide-fast.md) - a fused `from_halves` and an unboxed limb product, kept only if measured faster
- [Error-path workload](/quest/m2/ts-bench-error-path.md) - the benchmark prices `?` on a frame split across reads
- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - every value that drops in moq-net and kio drops where Rust drops it
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - moq-net's and kio's async code translates with Rust's lazy futures
- [Shared ownership](/quest/m2/ts-shared.md) - `Arc`/`Rc`/`Weak` keep exact counts, and locks and atomics become plain values
- [Collections](/quest/m2/ts-collections.md) - maps, sets, and queues keep Rust's key semantics
- [Tree-shaking spike](/quest/m2/ts-shake-spike.md) - classes, free functions, or free functions behind root facades, measured on speed, shaken size, and caller code
- [Tree-shakable runtime](/quest/m2/ts-runtime-shake.md) - `rt`'s adaptors and formatters become individually exported functions, kept only if no slower
- [Translate moq-net's model to TypeScript](/quest/m2/ts-model.md) - the driver grows from moq-net's codec to its model, for moq to compare against WASM
- [Delete the C++ generator's dead modes](/quest/m2/uniffi-cpp-dead-modes.md) - scaffolding mode and exception-style async go, with no consumer
- [Zero-copy byte returns](/quest/m2/ffi-bytes-return.md) - returned payloads reach the host without a copy, if the payload numbers justify it
- [Cheaper async](/quest/m2/ffi-async.md) - `RustFuture` exports cost fewer crossings, if the numbers show they matter
- [Namespaced FFI contract](/quest/m2/uniffi-contract.md) - bindings and a cdylib from different UniFFI forks refuse to load together
- [Drop checksums](/quest/m2/ffi-checksums.md) - bindings load without a checksum call per method
- [Smaller bindings](/quest/m2/ffi-code-size.md) - generated bindings shrink in every language, measured per template
