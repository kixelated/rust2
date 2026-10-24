# [M] Early cutoff for edit rebuilds

## Goal

A report says how often real moq-net edits leave
[rust-lang/rust#156940](https://github.com/rust-lang/rust/pull/156940)'s public API
hash unchanged, so their dependents could skip rebuilding, and how many seconds that
saves with incremental on and off. It then picks: carry #156940 on a custom
toolchain, help upstream, or stop. The other route,
[rust-lang/cargo#17439](https://github.com/rust-lang/cargo/issues/17439), is a design
discussion.

## Plan

Decided with the maintainer on 2026-10-24: measure real edits first, rather than
build. Re-scoped the same day after build-speed research: a wrapper that patches the
SVH and `SourceFile` hashes out of an otherwise identical rmeta is no longer a
candidate, since tinymold already ran that spike
([tinyhumansai/tinymold#13](https://github.com/tinyhumansai/tinymold/pull/13), ADR
0007) with the same result: identical objects, a different rmeta, and E0460
blocking reuse.

- Build #156940's `-Z public-api-hash` on a scratch toolchain from the pinned
  nightly, and replay moq-net's git history through it edit by edit.
- Count how often the public hash stays unchanged, how often only span shifts break
  it, and how often `async fn` bodies do. Report the seconds saved with incremental
  on and off side by side.
- Check determinism: with no source change, are moq's rmeta bytes stable across
  incremental sessions? headstart found def-ids encoded in a different order when
  loaded from the incremental cache, which would also defeat cargo#17439's output
  hashing.

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
- [headstart](https://github.com/PowderworksCode/headstart) emits early
  interface-only metadata for clean builds; not chosen for now.
- The cg_clif performance goal speeds up the backend, which is 6% of moq's edit, so
  it does not apply here.
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
- [ra-jit in the build benchmark](/quest/m3/bench-ra-jit.md) - skips the SVH cascade another way, by patching changed functions into the linked binary
