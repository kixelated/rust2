# [M] TypeScript translator benchmark

## Goal

`just ts bench` measures the same workloads four ways (native Rust, moq's WASM
build, moq's hand-written TypeScript, and [rust2-ts](/ts/README.md)'s
translation) in Node, Bun, and Chrome, plus each route's bundle size raw and
gzipped, and runs nightly in CI. Every translator spike and perf change states
its effect with these numbers.

## Plan

Decided with the maintainer on 2026-10-23: measure before choosing designs.
Spikes prototype and report; the report rescopes or deletes the quests that
follow.

- Workloads start with the codec moq's hand-written TS already benchmarks
  (`js/net/bench/varint.ts`, `lite-varint.ts`, `frames.ts`) at the moq revision
  `just ts moq` pins, and grow as the translator does.
- The differential harness already runs the wasm32 build beside the
  translation in one process; extend it with timing rather than adding a
  second harness.
- moq's WASM is `rs/moq-wasm` (wasm-bindgen, size-optimized), consume path
  only. Count boundary copies separately from compute.
- Report variance with an A/A control and record the engines' versions and the
  host. Never set a threshold up front; the report presents numbers.

Public API: none.
