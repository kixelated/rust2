# [S] A std Once in a static

## Goal

rust2-ts translates std's `Once`, read from a static's memory and called through
`call_once` and `call_once_force`, so crossbeam-epoch's own `OnceLock<Collector>` static
translates, proven by differential tests against native Rust.

## Plan

Left by [#255](https://github.com/moq-dev/rust2/pull/255) on 2026-10-29: statics read from memory now translate std's
`OnceLock` and `LazyLock`, but crossbeam-epoch keeps its own `OnceLock` over a bare `Once`,
which has no runtime counterpart. The model's census refuses it at
`crossbeam-epoch-0.9.21/src/default.rs:18`, once as the static's `Once` and once as
`Once::call_once`.

- An `rt.Once`, a state as wasm32's `no_threads` one is: incomplete, running, or
  complete. A `Once` read from memory is incomplete, as const evaluation leaves it, and any
  other state is refused.
- `call_once` from within its own function panics as wasm32's does, which `once.ts` can
  check beside `OnceLock`'s.

Public API: the translator's output.
