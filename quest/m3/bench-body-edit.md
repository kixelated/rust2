# [S] Body edits in the build benchmark

## Goal

The [build-speed benchmark](/bench/build/README.md) times moq edits to the body
of a function moq-relay calls, one sync and one async, in a moq-net submodule,
beside today's marker edit. The marker is a public function nothing calls,
appended to the crate root: rustc rebuilds every crate above it, while ra-jit
compiles nothing for it, so neither lever that skips work on a body edit is
measured on one.

## Plan

- ra-jit counts an `async fn` body as part of the signature (its `outline.rs`).
  By hand on 2026-10-25 (ra-jit d4f6027, moq at the corpus rev), sync body edits
  in `rs/moq-net/src/goaway.rs` stayed with the daemon, while two of three async
  ones went to cargo: "`new` is not compiled by ra-jit and rustc's copy is from
  before the edit". The report's per-run share says how often that happens.
- A private sync body edit in a submodule is also what relinking without
  rebuilding is built to cut off.
- A new scenario changes what every variant runs: land it on its own.

Public API: benchmark scenarios.

## Related

- [Relink without rebuilding](/quest/m3/relink.md) - the upstream lever the same edit decides
