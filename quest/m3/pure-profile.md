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

Public API: a new tool.
