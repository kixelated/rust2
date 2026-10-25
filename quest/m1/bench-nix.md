# [M] Nix builds in the build benchmark

## Goal

The [build-speed benchmark](/bench/build/README.md) measures moq's Nix release
builds (`nix build .#moq-relay` and a second package) in runner CPU time, so moq
can choose how they share compilations:

- crane as moq builds it after its layering fixes;
- crane with mbx as its `RUSTC_WRAPPER`, on a local store mounted into the
  sandbox with `extra-sandbox-paths`, as nixpkgs' ccache support does;
- crate2nix, one derivation per crate, shared through the Nix cache itself;
- cargo with mbx, as moq's CI builds today.

The report says which one moq should adopt, as a follow-up quest in moq.

## Plan

Decided with the maintainer on 2026-10-25: across moq's many CI and release
builds, each crate configuration compiles once and is reused, measured in runner
CPU time. Nix builds run sandboxed with no network, so they cannot reach mbx's
R2 store, and crane's two layers rebuild every dependency, or the whole
workspace, for changes that touch one crate. This bench decides between the
options rather than picking one up front.

- Pin moq after its crane fixes (moq-dev/moq `quest/m0/ci/crane-layers.md`),
  and rebuild moq's previous crane once from history for the before number.
- Scenarios, each with a cold and a warm cache: a clean build, a workspace edit
  (the existing moq-net edit), a `Cargo.lock` bump of one dependency, and a
  second package built after the first, which shows what the two share.
- Count what each option rebuilds as well as its time, and record the Nix
  evaluation time separately, since crate2nix evaluates about 400 derivations.
- crate2nix reimplements cargo's build logic: note every crate that needs an
  override (build scripts with `links`, proc macros, moq's kio `[patch]`).
- Nightly only, not on pull requests; the maintainer counts CPU time over wall
  clock.

Public API: benchmark variants.
