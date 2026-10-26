# m2: research

## Goal

Research work with a named consumer that is not waiting on it yet: the
TypeScript translator's spikes and features, the UniFFI fork's gated or
speculative work, and the driver's build-rule lint.

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

- [Lower bodies from MIR](/quest/m2/ts-mir.md) - the MIR spike's prototype becomes the only lowering, and the THIR one goes
- [Cheaper byte slices](/quest/m2/ts-byte-views.md) - splitting a `&[u8]` stops making a `Uint8Array` view per split
- [Wide-integer runtime off bigint](/quest/m2/ts-wide-bigint.md) - checked sums, 128-bit quotients, and comparisons with numbers stop going through `bigint`, kept only if measured faster
- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - every value that drops in moq-net and kio drops where Rust drops it
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - moq-net's and kio's async code translates with Rust's lazy futures
- [Shared ownership](/quest/m2/ts-shared.md) - `Arc`/`Rc`/`Weak` keep exact counts, and locks and atomics become plain values
- [Translate moq-net's model to TypeScript](/quest/m2/ts-model.md) - the driver grows from moq-net's codec to its model, for moq to compare against WASM
- [Monomorphization budget lint](/quest/m2/mono-budget.md) - rust2-driver fails a build whose generic instantiations exceed a budget, with the generic's span
- [Nightly MSVC and macOS](/quest/m2/uniffi-ci-msvc-macos.md) - the C and C++ fixtures compile nightly with MSVC and Apple Clang, at C++17 and C++20, and without exceptions or RTTI
- [Symlinked paths](/quest/m2/uniffi-symlink-path.md) - the fork builds when a consumer reaches it through a symlink, as on macOS's `/var`
- [Waiting: ubrn reads UniFFI 0.32](/quest/m2/ubrn-release.md) - the binding benchmark's browser runner pins a ubrn release, not an open PR
