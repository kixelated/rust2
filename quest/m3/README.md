# m3: deferred

## Goal

Work that waits on evidence or has no consumer yet: the build-speed subset
tracks, which the benchmark report rescopes before they start.

## Required

- [Pure-Rust profile](/quest/m3/pure-profile.md) - a checked profile with no build scripts and no C linking buys measured faster builds
- [Subset compiler](/quest/m3/subset-compiler.md) - a compiler or debug fast path for the pure subset, if the benchmark shows rustc leaves a gap it can close
