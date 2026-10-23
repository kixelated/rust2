# [L] Import UniFFI and moq's generators

## Goal

`uniffi/` holds a hard fork of UniFFI's core plus every generator moq uses
(Kotlin, Swift, Python, Go, Dart, C++, and C) in one Cargo workspace with a
committed lockfile. It publishes `rust2-uniffi`, `rust2-uniffi-core`,
`rust2-uniffi-macros`, and `rust2-uniffi-bindgen` to crates.io under one tag,
and the flake exposes each generator so moq pins one input. Bindings generated
from the fork are byte-identical to what moq generates today.

## Plan

Decided with the maintainer on 2026-10-13 in moq, and moved here on
2026-10-23 ([moq-dev/moq#6578](https://github.com/moq-dev/moq/pull/6578)):

- **Why**: every UniFFI minor release breaks external generators, so moq
  carried three generator forks plus a generator-only core fork, each pinned
  in about five places. The core also blocks byte copies, zero-copy returns,
  and namespaced external types.
- **Fresh import, not a GitHub fork or submodules**: uniffi-rs at 0.32.2
  (moq's generator-only fork is `kixelated/uniffi-rs` at `88d835d`),
  `kixelated/uniffi-bindgen-go` at `v0.10.0-kixelated.1+v0.32.0`,
  `kixelated/uniffi-dart` at `v0.3.1-kixelated.6+v0.32.0`, and
  `kixelated/uniffi-bindgen-cpp` at `v0.11.0-kixelated.5+v0.32.2` with its C
  backend. Record each import's upstream source and revision, and keep each
  component's license (uniffi-rs is MPL-2.0). Carried changes are listed in the
  fork's README per `CONTRIBUTING.md`.
- **moq's bindgen moves too**: `rs/uniffi-bindgen`, the Kotlin/Swift/Python
  CLI, and the Dart build that moq patches through `nix/uniffi-dart-*` become
  this workspace's generator binaries. Packaging scripts stay in moq.
- **Free to diverge**: the fork may change metadata, checksums, and the FFI
  contract when a consumer needs it; upstream changes are cherry-picked, not
  tracked. Offering a patch upstream needs maintainer approval each time.
  Give the contract its own namespace, so mixing with upstream UniFFI fails
  loudly at load time.
- **Publishing**: crates.io crates with no git dependencies, so crates.io
  moq-ffi still builds. Add release-plz (as moq does) with this quest, since
  it is the first published crate.
- **CI** builds every generator and runs the fixture suite, including the
  unlocked `cargo install` resolve that bit moq's 0.32 port.
- Prove byte-identical output by generating moq-ffi's bindings in every
  language with the fork and diffing against moq's `main`.

This is the first crate here, so it also adds the Cargo workspace and the cargo
steps of `just check` unless the benchmark landed them first.

Public API: new `rust2-uniffi*` crates and generator binaries. moq's switch is
its own quest there.
