# m1: consumers waiting

## Goal

The work moq is waiting on: the UniFFI fork its bindings switch to, the exporter
that may replace it, and the fork's perf work moq planned.

## Plan

Decided with the maintainer on 2026-10-24: an [exporter](/ffi/README.md) that turns a
crate's `pub` API into bindings, with no attributes, may replace the UniFFI fork. The fork
stays in maintenance and keeps building moq's bindings; its feature quests, here and in
m2, pause until the [decision](/quest/m1/ffi-export-decision.md) the spike's report asks
for, which decides whether each is folded into the exporter. The binding benchmark
continues as the exporter's baseline.

## Required

- [Waiting: publish the UniFFI fork](/quest/m1/uniffi-release.md) - the fork publishes to crates.io under `rust2-*` names, once the maintainer decides it should
- [Binding benchmark](/quest/m1/ffi-bench.md) - moq's `bench-ffi` moves here and measures every wrapper against the fork, nightly
- [Waiting: the exporter decision](/quest/m1/ffi-export-decision.md) - the maintainer decides from the spike's report whether the exporter replaces the UniFFI fork
- [Exporter](/quest/m1/ffi-export.md) - moq's bindings exported from its crates' `pub` API, if adopted
- [Payload bytes](/quest/m1/ffi-bytes.md) - payloads cross the FFI in bulk copies, proven in every wrapper
- [Kotlin calls](/quest/m1/kt-ffi-calls.md) - Kotlin calls cost about what Python's and Go's do, starting from upstream's JNI bindgen
- [Waiting: moq's poll-native async](/quest/m1/moq-poll-migrate.md) - condition: moq's `ffi-poll-migrate` quest has landed
- [Foreign poll for hot reads](/quest/m1/ffi-poll-protocol.md) - track and group reads are a sync poll plus a registered waker in every wrapper, and a cancel never loses a frame
