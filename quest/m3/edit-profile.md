# [S] Where an edit rebuild goes

## Goal

The [build-speed benchmark](/bench/build/README.md) attributes moq-relay's
moq-net edit rebuild (8.5 s) by crate rebuilt and by rustc stage and query, and
states what upstream's in-flight work (Cranelift, the parallel frontend,
relinking without rebuilding) removes from it. The report decides whether the
[subset compiler](/quest/m3/subset-compiler.md) has a gap to close.

## Plan

Decided with the maintainer on 2026-10-23: measure before committing to an XL
compiler. A leaf edit takes 3.3 s, 0.7 s of it linking; the moq-net edit takes
8.5 s against 120 s clean. Extend the benchmark's existing attribution rather
than adding a tool.

Public API: none.
