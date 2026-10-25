# [M] Cheaper async for RustFuture exports

## Goal

An async export that stays on UniFFI's `RustFuture` (connect, accept, session
status, and the other exports [foreign poll](/quest/m1/ffi-poll-protocol.md)
leaves alone) costs fewer FFI crossings per call in every generator, proven on
the [binding benchmark](/bench/ffi/README.md), with no change to the
bindings' async signatures.

## Plan

Decided with the maintainer on 2026-10-23: its own quest, after foreign poll,
so it reuses that waker design and measures only what remains.

- UniFFI 0.32 spends at least five crossings on a future that is ready on its
  first poll (call, poll, continuation, complete, free), plus a poll and a
  callback per extra wake. Measure the share these exports actually take
  before changing the protocol; if it is noise, delete this quest and record
  the numbers.
- The fork may change the FFI contract here; keep it under the fork's own
  contract namespace so it never mixes with upstream UniFFI.

Public API: none expected.

## Required

- [Exporter spike](/quest/m1/ffi-export-spike.md) - this quest is paused until the spike reports, and may be folded into the exporter
- [Foreign poll for hot reads](/quest/m1/ffi-poll-protocol.md) - the waker design this reuses
