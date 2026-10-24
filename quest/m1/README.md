# m1: consumers waiting

## Goal

The work moq is waiting on: the UniFFI fork its bindings switch to and the fork's
perf work moq planned.

## Required

- [Waiting: publish the UniFFI fork](/quest/m1/uniffi-release.md) - the fork publishes to crates.io under `rust2-*` names, once the maintainer decides it should
- [Payload bytes](/quest/m1/ffi-bytes.md) - payloads cross the FFI in bulk copies, proven in every wrapper
- [Kotlin calls](/quest/m1/kt-ffi-calls.md) - Kotlin calls cost about what Python's and Go's do, starting from upstream's JNI bindgen
- [Waiting: moq's poll-native async](/quest/m1/moq-poll-migrate.md) - condition: moq's `ffi-poll-migrate` quest has landed
- [Foreign poll for hot reads](/quest/m1/ffi-poll-protocol.md) - track and group reads are a sync poll plus a registered waker in every wrapper, and a cancel never loses a frame
