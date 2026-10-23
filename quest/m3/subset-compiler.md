# [XL] Subset compiler

## Goal

A compiler, interpreter, or debug fast path for the pure-Rust subset builds
moq-scale code measurably faster than rustc on the
[build-speed benchmark](/bench/README.md), for the scenario the report shows
matters most (likely incremental debug rebuilds).

## Plan

Decided with the maintainer on 2026-10-23: plan it now, blocked on the
benchmark, and rescope it from the report before starting. It may share a
frontend with [the TypeScript translator](/quest/m2/ts.md).

Research (2026-10-23): no "tcc for Rust" exists; mrustc is a bootstrap tool and
gccrs cannot compile `core`. Upstream is attacking the same costs for every
crate (Cranelift, TPDE, the parallel frontend, Relink-Don't-Rebuild), and the
dominant typeck and trait-solving cost does not shrink unless the subset bans
language features. The report must show a gap upstream will not close before
this starts; otherwise delete this quest.

Public API: a new tool.
