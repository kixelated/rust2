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

- [Cheaper byte slices](/quest/m2/ts-byte-views.md) - splitting a `&[u8]` stops making a `Uint8Array` view per split
- [Smaller output from MIR](/quest/m2/ts-mir-size.md) - the bench's bundles shrink back to THIR's sizes, coroutine classes first
- [Time the minified bundles](/quest/m2/ts-bench-minified.md) - `just ts bench` times what it measures for size, as consumers ship it
- [Wide-integer runtime off bigint](/quest/m2/ts-wide-bigint.md) - checked sums, 128-bit quotients, and comparisons with numbers stop going through `bigint`, kept only if measured faster
- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - every value that drops in moq-net and kio drops where Rust drops it
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - moq-net's and kio's async code translates with Rust's lazy futures
- [Shared ownership](/quest/m2/ts-shared.md) - `Arc`/`Rc`/`Weak` keep exact counts, and locks and atomics become plain values
- [Translate char and vec![x]](/quest/m2/ts-char-vec.md) - `char` values and `vec![x]` translate, unblocking moq-net's refused sites
- [Firefox in the TS benchmark](/quest/m2/ts-bench-firefox.md) - SpiderMonkey joins Chrome, Bun, and Node in `just ts bench`
- [Translate moq-net's model to TypeScript](/quest/m2/ts-model.md) - the driver grows from moq-net's codec to its model, for moq to compare against WASM
- [Symlinked paths](/quest/m2/uniffi-symlink-path.md) - the fork builds when a consumer reaches it through a symlink, as on macOS's `/var`
- [Waiting: ubrn reads UniFFI 0.32](/quest/m2/ubrn-release.md) - the binding benchmark's browser runner pins a ubrn release, not an open PR
