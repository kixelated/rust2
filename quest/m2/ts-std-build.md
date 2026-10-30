# [S] Check std once per toolchain

## Goal

rust2-ts checks std from source in every cold target directory, through `-Zbuild-std`, so
its translations see std's un-inlined MIR: the prebuilt std's optimized MIR changes
snapshots and the census and refuses `LocalKey`'s methods (measured in
[#237](https://github.com/kixelated/rust2/pull/237)). Check std once per toolchain
instead, as a Nix derivation the translator takes as its sysroot, so CI's runners, which
keep `/nix/store`, check it only when the nightly changes. Snapshots and moq's census stay
byte-identical.

## Plan

Decided on #237 on 2026-10-29.

- A derivation checks std with `-Zbuild-std` from the toolchain's `rust-src`, offline
  from its vendored crates, with the flags translation uses, and keeps the `.rmeta` files
  as a sysroot.
- The dev shell and the `ts` package point the translator at it; the wrapper passes it to
  rustc as `--sysroot` for the target's crates only.
- Without one, outside Nix, the translator falls back to `-Zbuild-std`, from one shared
  list of flags; a nightly job runs the suite that way, so the two stay identical.
- Measure cold and warm CPU-s for a fixture and for the suite.

Public API: none; the `ts` package and dev shell set a new environment variable.
