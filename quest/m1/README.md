# m1: consumers waiting

## Goal

The work moq is waiting on: the UniFFI fork its bindings switch to and the fork's
perf work moq planned, and the build-speed benchmark that gates every subset
change.

## Required

- [Import UniFFI and moq's generators](/quest/m1/uniffi.md) - one workspace builds UniFFI's core and every generator moq uses, published as `rust2-uniffi*` crates under one tag
- [Binding benchmark](/quest/m1/ffi-bench.md) - moq's `bench-ffi` moves here and measures every wrapper against the fork, nightly
- [Payload bytes](/quest/m1/ffi-bytes.md) - payloads cross the FFI in bulk copies, proven in every wrapper
- [Kotlin calls](/quest/m1/kt-ffi-calls.md) - Kotlin calls cost about what Python's and Go's do, starting from upstream's JNI bindgen
- [Waiting: moq's poll-native async](/quest/m1/moq-poll-migrate.md) - condition: moq's `ffi-poll-migrate` quest has landed
- [Foreign poll for hot reads](/quest/m1/ffi-poll-protocol.md) - track and group reads are a sync poll plus a registered waker in every wrapper, and a cancel never loses a frame
- [Build-speed benchmark](/quest/m1/bench.md) - a reproducible before/after measurement of moq and a pure-Rust corpus, run nightly
