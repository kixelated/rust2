# [S] Rebuild a target directory when its std changes

## Goal

A target directory that [rust2-ts](/ts/README.md) translated against one std sysroot
rebuilds what depends on std once `RUST2_TS_SYSROOT` names another, so no change to
`ts/src/profile.toml` or the flake fails with rustc's E0460 until the directory is cleaned.

## Plan

Found by [#237](https://github.com/moq-dev/rust2/pull/237) on 2026-10-29: changing only
`profile.toml`'s core, alloc, and std lines on the same nightly gives a new sysroot, but
cached crates stay built against the old one.

- The likely root cause: the wrapper adds `--sysroot` itself, so nothing cargo fingerprints
  names the sysroot. The shared flags rebuild everything only because they are in the
  profile, which cargo hashes. Confirm with a reproduction before fixing.
- Fix it where cargo looks: the sysroot's identity (its store path, or a hash of its
  contents) in something every unit's fingerprint and `-C metadata` cover, so two
  sysroots keep separate artifacts rather than evicting each other. The profile's
  rustflags are the obvious place, as for the shared flags.
- Pitfalls: cargo checks dep-info env dependencies only for path packages, so declaring
  `RUST2_TS_SYSROOT` that way misses registry crates; `RUSTFLAGS` overrides
  `target.<triple>.rustflags` (why #237 dropped it); build scripts and proc macros must keep
  the host std; and cargo caches `rustc -vV` in `target/.rustc_info.json` by paths and
  mtimes, not by our environment.
- Test it the way it failed: one target directory, two sysroots that differ only in std's
  flags, no clean between.

m2 because it breaks the translator's own fixtures and moq's `just ts` runs whenever a
rust2 bump changes only std's flags, but loudly, and a clean recovers.

Public API: none.
