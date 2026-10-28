# [M] Driving sessions and origins

## Goal

A binding runs moq's sessions and origins: each runs a `Driver` polled with a time and a
`kio::Waiter`, and the exporter gives bindings a way to drive it that every generator can
use.

## Plan

Open: export that poll, generate a thread that drives it, or export moq-tokio's runtime
helpers. Go's cancel contract (moq's `go/wrapper/doc.go` ends the stream) is decided
with it. `time::run` at `origin::Driver` was left out of
[#126](https://github.com/kixelated/rust2/pull/126) because it runs on an ambient runtime
C does not have.

Public API: the exporter's C ABI for drivers.
