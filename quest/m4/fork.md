# [XL] Waiting: a rustc and cargo fork

## Goal

Whether rust2 should carry its own rustc and cargo fork, installed as a rustup
toolchain so moq needs no code changes. This quest is the evidence ledger.
Condition: the reasons to fork add up to enough to pay for maintaining one,
which is the maintainer's call. Later reports append their data points here.

## Reasons to fork

Each names what the fork changes, the measured gain on moq, and its source.

- **Early cutoff without positions in dependents' bodies.** Keep panic and
  debuginfo locations out of the generic and inline bodies dependents compile
  (for example, in a side table), keep the SVH and DefIndex stable when the API
  hash holds (as in [rust-lang/rust#158844](https://github.com/rust-lang/rust/issues/158844)),
  and have cargo skip those dependents, including under pipelining
  ([rust-lang/cargo#17439](https://github.com/rust-lang/cargo/issues/17439)).
  Gain: a sound cutoff holds on about 65 of 250 non-test moq-net edits in dev
  (26%), up from 18 (7%), but on only 7 of 84 async edits
  ([kixelated/rust2#62](https://github.com/kixelated/rust2/pull/62)). Each saves
  at most about 56% of the edit: the dependents' share of its critical path,
  minus the final link ([relink](/quest/m3/relink.md)).

## Not reasons

- **A drop-in `RUSTC_WRAPPER` cutoff**, declined on 2026-10-25
  ([kixelated/rust2#70](https://github.com/kixelated/rust2/pull/70)):
  - it is unsound on stock rustc: a skipped dependent keeps its dependency's old
    SVH, so the next crate fails with E0460, and its rmeta encodes the
    dependency's DefIndex raw, which shifts;
  - caching, GC, and dedup are already in mbx, and cargo's cross-workspace cache
    goal is in progress;
  - its sound cutoff holds on 7% of moq-net edits
    ([kixelated/rust2#62](https://github.com/kixelated/rust2/pull/62)).

## Plan

- The fork carries its changes itself rather than working through upstream.
- Measure each reason on moq's edits in the
  [build-speed benchmark](/bench/build/README.md).
- Code that follows the subset's rules (poll-first, thin generics) only gains
  more; it is never required.

Public API: none until the fork has a consumer.
