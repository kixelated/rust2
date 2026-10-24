# m2: research

## Goal

Research work with a named consumer that is not waiting on it yet: the
TypeScript translator's spikes and features, and the UniFFI fork's gated or
speculative work.

## Plan

Decided with the maintainer on 2026-10-23: the translator measures before it
builds. The benchmark and census come first; each spike's report rescopes or
deletes the quests that follow it.

## Required

- [Cheaper byte slices](/quest/m2/ts-byte-views.md) - splitting a `&[u8]` stops making a `Uint8Array` view per split
- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - every value that drops in moq-net and kio drops where Rust drops it
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - moq-net's and kio's async code translates with Rust's lazy futures
- [Shared ownership](/quest/m2/ts-shared.md) - `Arc`/`Rc`/`Weak` keep exact counts, and locks and atomics become plain values
- [Collections](/quest/m2/ts-collections.md) - maps, sets, and queues keep Rust's key semantics
- [Translate moq-net's model to TypeScript](/quest/m2/ts-model.md) - the driver grows from moq-net's codec to its model, for moq to compare against WASM
- [Zero-copy byte returns](/quest/m2/ffi-bytes-return.md) - returned payloads reach the host without a copy, if the payload numbers justify it
- [Cheaper async](/quest/m2/ffi-async.md) - `RustFuture` exports cost fewer crossings, if the numbers show they matter
- [Namespaced FFI contract](/quest/m2/uniffi-contract.md) - bindings and a cdylib from different UniFFI forks refuse to load together
- [Drop checksums](/quest/m2/ffi-checksums.md) - bindings load without a checksum call per method
- [Smaller bindings](/quest/m2/ffi-code-size.md) - generated bindings shrink in every language, measured per template
