# m1: consumers waiting

## Goal

The work moq is waiting on: the UniFFI fork its bindings switch to, and the
build-speed benchmark that gates every subset change.

## Required

- [Import UniFFI and moq's generators](/quest/m1/uniffi.md) - one workspace builds UniFFI's core and every generator moq uses, published as `rust2-uniffi*` crates under one tag
- [Build-speed benchmark](/quest/m1/bench.md) - a reproducible before/after measurement of moq and a pure-Rust corpus, run nightly
