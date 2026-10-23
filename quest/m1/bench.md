# [M] Build-speed benchmark

## Goal

`just bench` reproducibly measures Rust build time on pinned revisions of moq
and a small pure-Rust corpus, and runs nightly in CI. It is the before/after
gate for every build-speed change here: a change states its effect with this
benchmark's numbers, and the report attributes where the time goes.

## Plan

Decided with the maintainer on 2026-10-23: the subset tracks (a pure-Rust
profile and an own subset compiler) both wait on this, and this report rescopes
them.

- Corpus: moq at pinned revisions, plus a few pure-Rust crates with no build
  scripts and no C linking, also pinned. A corpus bump is its own change, so
  numbers compare across runs.
- Scenarios: clean debug build, `cargo check`, and an incremental edit in a
  leaf crate and in a widely depended-on crate (moq-net in moq).
- Tooling: hyperfine for wall time, plus `cargo build --timings` and
  `-Zself-profile` (on a pinned nightly) to attribute time to build scripts,
  proc macros, frontend, codegen, and linking. Prefer existing tools; rustc-perf's
  collector compares compilers but cannot compare cargo-level caching.
- Report variance with an A/A control, and record the host. Never set a
  threshold up front; the report presents numbers and the maintainer decides.
- Research for the report (2026-10-23): most gains land upstream for every
  crate, not just a subset (LLD by default since Rust 1.90, wild and mold, the
  parallel frontend, TPDE, Relink-Don't-Rebuild, `-Zcache-proc-macros`). A
  subset mainly buys hermetic, content-addressed caching and pure proc
  macros. Measure those upstream levers as baselines first.

Public API: none. Adds the Cargo workspace if it is the first crate.
