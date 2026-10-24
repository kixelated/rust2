# [XL] Subset compiler

## Goal

A compiler, interpreter, or debug fast path for the pure-Rust subset builds
moq-scale code measurably faster than rustc on the
[build-speed benchmark](/bench/build/README.md), for the scenario the report shows
matters most (likely incremental debug rebuilds).

## Plan

Decided with the maintainer on 2026-10-23: plan it now, blocked on the
benchmark, and rescope it from the report before starting. It may share a
frontend with [the TypeScript translator](/ts/README.md).

Research (2026-10-23): no "tcc for Rust" exists; mrustc is a bootstrap tool and
gccrs cannot compile `core`. Upstream is attacking the same costs for every
crate (Cranelift, TPDE, the parallel frontend, Relink-Don't-Rebuild), and the
dominant typeck and trait-solving cost does not shrink unless the subset bans
language features. The report must show a gap upstream will not close before
this starts; otherwise delete this quest.

Benchmark ([run](https://github.com/kixelated/rust2/actions/runs/40040916279), 2026-10-23, 4-vCPU x86_64 runner): moq-relay
rebuilds in 3.3 s after a leaf edit and 8.5 s after a moq-net edit, against
120 s clean. Linking the relay is 0.7 s of the leaf edit (mold and wild cut the
edit about 10%), and the rest is one crate's frontend and codegen. Cranelift
cuts moq's clean build 11.5% and the pure crates' 12-17%, but not moq's edits;
`--jobs-frontend=8` speeds the pure crates' checks 30-36% yet slows moq 3-5% on
4 cores, where cargo already fills them.

Public API: a new tool.
