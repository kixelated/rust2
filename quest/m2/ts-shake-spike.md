# [M] Tree-shaking spike

## Goal

A report recommends the default shape [rust2-ts](/ts/README.md) emits for translated
types, from three built and measured:

1. Classes everywhere, today's baseline.
2. Free functions everywhere: plain objects plus exported functions.
3. Free functions inside, with a thin class facade only on root types.

Each is measured on `just ts bench` speed, on bundle size, and on what caller code
looks like. Bundle size is a Bun production build (`bun build --minify`, which
tree-shakes) of a small consumer that imports part of the API, raw and gzipped,
using the benchmark's existing Bun setup. The winner lands with a CI tree-shake test: a
consumer importing one function, asserting nothing unused survives.

## Plan

Decided with the maintainer on 2026-10-24: spike all three and measure. The
maintainer wants rust2-ts to produce libraries that don't force callers into
awkward code. A per-crate `ts.toml` option was considered and not chosen for the
spike. The ranking is the translator's ([design](/ts/README.md#design)):
correctness, then speed, then bundle size, which counts only the final minified
JS, so type-only constructs are free.

Survey how comparable libraries trade this off: RxJS's move to pipeable operators,
Zod 4's `zod/mini`, Valibot, Effect's dual APIs, date-fns, and wasm-bindgen's
output.

Public API: the translator's output shape for types, if the default changes.

## Related

- [Tree-shakable runtime](/quest/m2/ts-runtime-shake.md) - the runtime goes free functions whatever this picks
