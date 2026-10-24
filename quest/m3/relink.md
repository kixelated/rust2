# [XS] Relink without rebuilding

## Goal

Upstream's relinking without rebuilding reaches the pinned nightly, and the
[build-speed benchmark](/bench/build/README.md) measures it as a nightly
variant on moq's edits. Check with `rustc -Z help` (a public API hash, as in
[rust-lang/rust#156940](https://github.com/rust-lang/rust/pull/156940)) and
`cargo -Z help` (an early cutoff, as in
[rust-lang/cargo#17439](https://github.com/rust-lang/cargo/issues/17439)).

## Plan

It is the one upstream lever the moq-net edit leaves room for. The benchmark's
attribution puts two thirds of that edit's critical path in crates rebuilt only
because moq-net changed; Cranelift, the parallel frontend, and the faster
linkers each move the edit within noise.

Status on 2026-10-24: neither flag is on nightly 2026-10-21. The 2025H2 project
goal will not complete; rust#156940 is a draft, and cargo's early cutoff is a
design discussion.

Public API: a benchmark variant.
