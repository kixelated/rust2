# [XL] Waiting: early cutoff on a rustc and cargo fork

## Goal

A prototype rustc and cargo fork, installed as a rustup toolchain so no code changes
are needed, that skips rebuilding a dependent when its dependency's public API hash
holds. It is measured on moq's edits in the
[build-speed benchmark](/bench/build/README.md). Condition:
[the semantic API hash](/quest/m3/api-cutoff.md) shows the hash holds on enough of
moq's edits to pay for a fork.

## Plan

Decided with the maintainer on 2026-10-25, after research into a drop-in cache like mbx:

- **Not a drop-in wrapper.** Nothing a `RUSTC_WRAPPER` could add is both sound and
  missing from mbx, kache, or upstream cargo:
  - caching, GC, and dedup are in mbx;
  - cargo's 2026 cross-workspace cache goal is in progress;
  - a wrapper cutoff is unsound, because a skipped dependent keeps its dependency's
    old SVH, so the next consumer fails with E0460, and it may hold inlined code from
    the old version.
- **A fork, not upstream.** The prototype carries the rustc changes the cutoff needs
  itself, without working through upstream. These are the hash in the crate header,
  no upstream monomorphizations, and gated private data, as in
  [rust-lang/rust#156940](https://github.com/rust-lang/rust/pull/156940)'s PR series
  ([rust-lang/rust#158844](https://github.com/rust-lang/rust/issues/158844)). The cargo
  side skips dependents, and must handle pipelining
  ([rust-lang/cargo#17439](https://github.com/rust-lang/cargo/issues/17439)).
- **What it can save:** at most about 56% of the moq-net edit. 65% of that edit's
  critical path is dependents, minus the final link. rust#156940's hash held on only
  4 of 171 edits, mostly because `async fn` bodies are in rmeta. moq's poll-first work
  and the semantic hash decide whether that improves.
- **Code that follows the subset's rules** (poll-first, thin generics) only hits more
  often; it is never required.

Public API: none until the prototype has a consumer.
