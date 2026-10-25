# m1: consumers waiting

## Goal

The work moq is waiting on: the UniFFI fork its bindings switch to, the exporter
spike that may replace it, and the fork's perf work moq planned.

## Plan

Decided with the maintainer on 2026-10-24: an [exporter](/quest/m1/ffi-export-spike.md)
that turns a crate's `pub` API into bindings, with no attributes, may replace the UniFFI
fork. The fork stays in maintenance and keeps building moq's bindings; its feature
quests, here and in m2, pause until the spike reports, which decides whether each is
folded into the exporter. The spike sits in m1 because it blocks them. The binding
benchmark continues as the exporter's baseline.

## Required

- [Waiting: publish the UniFFI fork](/quest/m1/uniffi-release.md) - the fork publishes to crates.io under `rust2-*` names, once the maintainer decides it should
- [Exporter spike](/quest/m1/ffi-export-spike.md) - moq-net's `pub` API exported to a C ABI and header by a rustc driver, with refusals counted and compared against moq's `rs/moq-c`
- [Payload bytes](/quest/m1/ffi-bytes.md) - payloads cross the FFI in bulk copies, proven in every wrapper
- [Kotlin calls](/quest/m1/kt-ffi-calls.md) - Kotlin calls cost about what Python's and Go's do, starting from upstream's JNI bindgen
- [Waiting: moq's poll-native async](/quest/m1/moq-poll-migrate.md) - condition: moq's `ffi-poll-migrate` quest has landed
- [Foreign poll for hot reads](/quest/m1/ffi-poll-protocol.md) - track and group reads are a sync poll plus a registered waker in every wrapper, and a cancel never loses a frame
