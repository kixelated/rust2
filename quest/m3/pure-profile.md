# [M] Pure-Rust build profile

## Goal

A checker proves a crate graph is pure Rust (no build scripts, no `-sys` crates
or C linking, and proc macros only where the benchmark shows they are safe to
cache), and builds use that guarantee for a measured speedup on the
[build-speed benchmark](/bench/build/README.md), such as hermetic content-addressed
caching or cached and sandboxed proc macros.

## Plan

Decided with the maintainer on 2026-10-23: build speed covers a profile, a
subset compiler, and the measurement that gates both. The benchmark report
rescopes this quest before it starts; rewrite it to the levers the numbers
favor.

Research (2026-10-23): the speedups that need purity are hermetic caching
(sccache, kache, and Bazel and buck2 rules break on undeclared build-script
inputs and native deps) and proc-macro caching or wasm sandboxing (watt, the
nightly `-Zcache-proc-macros`, whose purity is unchecked). Most real graphs use
`syn` derives and some `-sys` crate, so measure how much of moq's graph a
profile can cover.

Benchmark ([run](https://github.com/kixelated/rust2/actions/runs/40040916279), 2026-10-23, 4-vCPU x86_64 runner): a clean
moq-relay debug build takes 120 s, and aws-lc-sys's build script alone runs for
88 s of it (11 s to 99 s), so one C dependency sits on the critical path. Build
script runs are 20% of the build's unit-seconds; proc-macro crates compile in
6%, and running proc macros takes 1.3 s of 520 s rustc self-time, which is why
`-Zcache-proc-macros` measured within noise. Edits rerun no build script.

Public API: a new tool.
