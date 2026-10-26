# [XL] A subset compiler

## Goal

A report deciding whether a compiler for a stricter subset of Rust could build moq
faster than rustc can after moq adopts its own build rules. The subset is
poll-first code with no `async fn` in libraries, thin generics, and few proc
macros. The report names the subset, the speedup it predicts on moq's
clean build and moq-net edit, and how its output links with crates rustc builds.

## Plan

Decided with the maintainer on 2026-10-25: allow a custom compiler as long-term
research, after moq's
[poll-first](https://github.com/moq-dev/moq/blob/main/quest/m1/poll-first/README.md)
and thin-generics work.

The [edit attribution](https://github.com/kixelated/rust2/pull/22) closed the
earlier version. What a revival has to answer:

- **Most of the edit is rebuilding dependents:** 65% of the moq-net edit's
  critical path is crates rebuilt only because it changed. A public API hash
  removes that ([relink](/quest/m3/relink.md)), with no new compiler.
- **What is left scales with the crate:** expansion, resolution, crate-wide
  analysis, and incremental I/O. A subset compiler would have to make these
  incremental per item, as Zig's compiler does, to win.
- **Linking:** it can't read or write rustc's unstable metadata, so it must own
  whole crates, with a C ABI or rustc-built shims at the boundary.
- **Few proc macros buys little on edits:** the
  [macro cost report](https://github.com/kixelated/rust2/pull/64) found that
  running proc macros is about 5% of the moq-net edit, which
  `-Z cache-proc-macros` already removes, and compiling them is a clean-build
  cost. The largest share of macro output is tracing's `macro_rules!`,
  re-expanded in full on every rebuild.
- **Evidence to collect first:**
  - the [semantic API hash](/quest/m3/api-cutoff.md) replay;
  - the [monomorphization budget](https://github.com/kixelated/rust2/pull/65) numbers.

Public API: none.
