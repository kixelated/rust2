# [XS] No-C check in the build benchmark

## Goal

The [build-speed benchmark](/bench/build/README.md) fails if the `moq-pure-crypto`
corpus ([corpus.toml](/bench/build/corpus.toml)) gains a `links` key or a `cc` or `cmake` build dependency, so C cannot creep
back into moq-relay once moq adopts pure-Rust crypto.

## Plan

Decided with the maintainer on 2026-10-24, adopting
[#13](https://github.com/kixelated/rust2/pull/13)'s recommendations. This is not
the `pure` check, which forbids every build script: moq's graph keeps pure-Rust
ones (serde, proc-macro2, libc, rustls). It pairs with moq's crypto adoption.

Public API: none; a benchmark check.
