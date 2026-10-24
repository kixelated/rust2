# m3: deferred

## Goal

Work that waits on evidence or has no consumer yet: the build-speed tracks,
each a measurement that decides whether its follow-up lands.

## Required

- [Pure-Rust crypto](/quest/m3/pure-crypto.md) - moq-relay on graviola and RustCrypto instead of aws-lc, measured on build time and throughput
- [Median for short scenarios](/quest/m3/bench-median.md) - sub-10 s benchmark numbers resist outliers
- [Where an edit rebuild goes](/quest/m3/edit-profile.md) - attribute the moq-net edit and what upstream removes from it
- [Subset compiler](/quest/m3/subset-compiler.md) - a compiler or debug fast path for the pure subset, if the benchmark shows rustc leaves a gap it can close
