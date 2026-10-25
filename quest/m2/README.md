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

The fork's feature quests folded into the [exporter](/quest/m1/ffi-export.md);
[m1](/quest/m1/README.md#plan) records why.

## Required

- [MIR's codec as fast as THIR's](/quest/m2/ts-mir-codec.md) - the codec from MIR no slower than from THIR in any engine
- [Lower bodies from MIR](/quest/m2/ts-mir.md) - the MIR spike's prototype becomes the only lowering, and the THIR one goes
- [Stable names](/quest/m2/ts-stable-names.md) - generic instances and trait-impl methods always carry their suffix, so callers' names never shift
- [Struct update on a std range](/quest/m2/ts-range-update.md) - `Range { start, ..r }` reads its base, or is refused with a span, instead of dropping it or panicking the driver
- [Cheaper byte slices](/quest/m2/ts-byte-views.md) - splitting a `&[u8]` stops making a `Uint8Array` view per split
- [Wide-integer fast paths](/quest/m2/ts-wide-fast.md) - a fused `from_halves` and an unboxed limb product, kept only if measured faster
- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - every value that drops in moq-net and kio drops where Rust drops it
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - moq-net's and kio's async code translates with Rust's lazy futures
- [Shared ownership](/quest/m2/ts-shared.md) - `Arc`/`Rc`/`Weak` keep exact counts, and locks and atomics become plain values
- [Collections](/quest/m2/ts-collections.md) - maps, sets, and queues keep Rust's key semantics
- [Tree-shakable runtime](/quest/m2/ts-runtime-shake.md) - `rt`'s adaptors and formatters become individually exported functions, kept only if no slower
- [Translate moq-net's model to TypeScript](/quest/m2/ts-model.md) - the driver grows from moq-net's codec to its model, for moq to compare against WASM
- [Nightly MSVC and macOS](/quest/m2/uniffi-ci-msvc-macos.md) - the C and C++ fixtures compile nightly with MSVC and Apple Clang, at C++17 and C++20, and without exceptions or RTTI
- [Symlinked paths](/quest/m2/uniffi-symlink-path.md) - the fork builds when a consumer reaches it through a symlink, as on macOS's `/var`
- [Waiting: ubrn reads UniFFI 0.32](/quest/m2/ubrn-release.md) - the binding benchmark's browser runner pins a ubrn release, not an open PR
