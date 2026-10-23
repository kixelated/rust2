# m1: consumers waiting

## Goal

The work moq is waiting on: publishing the UniFFI fork its bindings switch to,
and the build-speed benchmark that gates every subset change.

## Required

- [Waiting: release secrets](/quest/m1/uniffi-release.md) - the release workflow can publish the UniFFI fork to crates.io
- [Build-speed benchmark](/quest/m1/bench.md) - a reproducible before/after measurement of moq and a pure-Rust corpus, run nightly
