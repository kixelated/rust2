# [M] Early cutoff for edit rebuilds

## Goal

A report says what share of real moq-net edits could skip rebuilding their
dependents, and how many seconds that saves, under each rule below. It then picks
how rust2 gets early cutoff without waiting on
[rust-lang/cargo#17439](https://github.com/rust-lang/cargo/issues/17439): a
hash-patching `RUSTC_WRAPPER`, carrying
[rust-lang/rust#156940](https://github.com/rust-lang/rust/pull/156940) on a custom
toolchain, or helping upstream.

## Plan

Decided with the maintainer on 2026-10-24: measure real edits first, rather than
build.

Replay moq-net's recent git history edit by edit. For each edit, check whether
dependents could be skipped under each rule:

1. The upstream rmeta is identical apart from the SVH and the `SourceFile` hash
   fields, which is what a wrapper could patch.
2. #156940's `-Z public-api-hash`, cherry-picked onto a scratch build of the
   pinned nightly.

Report edits to `async fn` bodies separately.

Research (2026-10-24, rustc at the pinned nightly 7df81381d):

- Dependents rebuild for two reasons: cargo's input-based fingerprints, and
  rustc's SVH. The SVH hashes all of the HIR, bodies included, and every
  downstream crate embeds it in `crate_deps`.
- Reusing an old dependent fails with E0460 at crate loading (`locator.rs`
  `crate_matches`). Symbols and object code are unaffected: a rebuilt dependent
  differs only in 32 hash bytes.
- Body edits also change rmeta's `SourceFile` records. Line shifts move later
  spans, and new closures shift DefIndex.
- rmeta always encodes coroutine MIR, so every `async fn` body is part of the
  interface.
- No build tool (Bazel rules_rust, buck2, sccache) gets early cutoff for Rust
  today.
- #156940 is a draft, waiting on its author, and absent from the pinned nightly.

Research on [mbx](https://github.com/jdx/mr-boxington) (Mr Boxington, MIT, a cargo
and rustc wrapper cache), 2026-10-24:

- mbx doesn't cut off dependents. Its keys hash each `--extern` artifact's bytes,
  and for edited crates it relies on private rustc incremental state. Its
  maintainers reproduced E0460 and rejected SVH patching as unsound in
  [jdx/mr-boxington#645](https://github.com/jdx/mr-boxington/pull/645) (closed
  unmerged): rustc's metadata format is unstable, and telling interface edits from
  body edits needs span-aware normalization.
- Their numbers: with incremental on, dependents cost about 1.1 s of a 1.7 s edit,
  and rustc incremental already cut one dependent from about 2.4 s to about 0.8 s.
- So the spike measures with incremental on, as the build benchmark does, and
  counts only what incremental leaves. Report incremental on and off side by side.
- Classify each edit against mbx's list of interface-relevant bodies: generic,
  `#[inline]`, and `const fn` bodies; `impl Trait` hidden types; new impls; plus
  `async fn` bodies, whose coroutine MIR is always encoded.
- A cheap first data point: `mbx analyze` charges each dependent's rebuild to the
  crate where the edit started. Run it on a few real moq-net edits before building
  anything.
- rust2's edge: its own rustc driver could define an interface hash for the
  pure-Rust path, which a wrapper cache can't.

The upper bound on the benchmark's moq-net edit is 3.9 s of 6.95 s. Its appended
non-generic function overstates the benefit, which is why the report replays real
edits.

Public API: none.

## Related

- [Relink without rebuilding](/quest/m3/relink.md) - waits for the same lever upstream; this spike may change its plan
