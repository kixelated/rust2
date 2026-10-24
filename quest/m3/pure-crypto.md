# [M] Pure-Rust crypto for moq-relay

## Goal

A report measures moq-relay with its crypto moved off aws-lc-rs onto existing
pure-Rust providers, so its graph compiles no C: clean build time on the
[build-speed benchmark](/bench/build/README.md), plus AEAD, handshake, and relay
throughput against aws-lc. The maintainer decides from it whether moq adopts the
swap.

## Plan

Decided with the maintainer on 2026-10-23: this replaces the pure-Rust profile.
The benchmark ([run](https://github.com/kixelated/rust2/actions/runs/40040916279))
puts build script runs at 89 of moq's 445 unit-seconds, 88 of them aws-lc-sys's
alone on the critical path of a 120 s build; every other build script totals
about 1 s, and edits rerun none. So the lever is aws-lc-rs, not `build.rs`:
pure-Rust crates such as proc-macro2, serde, and libc keep trivial build
scripts, and the line that matters is no compiled C (`links`, `cc`).
Proc-macro caching (1.3 s) and hermetic caching are dropped until a measurement
asks for them.

- Never hand-roll primitives. graviola is the rustls provider (AEAD, key
  exchange, signatures); RustCrypto fills what graviola lacks, including
  jsonwebtoken's `rust_crypto` backend.
- aws-lc-rs reaches moq-relay directly through moq-quic, moq-auth, and
  moq-relay, and through rustls, rustls-webpki, rcgen (moq-tokio, rustls-acme),
  and jsonwebtoken. Every one moves, or the report names what blocks it.
- Prototype on a moq branch; the benchmark measures it as a variant of the moq
  corpus. Set no threshold up front.
- If moq adopts it, the moq corpus joins the benchmark's no-C check, so C
  cannot creep back in.

Public API: none here; the moq changes are moq's.
