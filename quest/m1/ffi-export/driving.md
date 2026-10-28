# [M] Driving sessions and origins

## Goal

A binding runs moq's sessions and origins: each runs a `Driver` polled with a time and a
`kio::Waiter`, and the exporter gives bindings a way to drive it that every generator can
use.

## Plan

Decided with the maintainer on 2026-10-27: the C ABI exports the `Driver`'s poll, and each
language drives it from its own event loop. No generated thread, and no moq-tokio helpers,
which would tie bindings to tokio. Go's cancel contract (moq's `go/wrapper/doc.go` ends the
stream) is decided with it. `time::run` at `origin::Driver` was left out of
[#126](https://github.com/kixelated/rust2/pull/126) because it runs on an ambient runtime
C does not have, and stays out.

Public API: the exporter's C ABI for drivers.
