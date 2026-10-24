# [M] Move moq's binding benchmark to rust2

## Goal

`just bench-ffi` runs here, nightly in CI: it builds moq-ffi at a pinned moq
revision against [the fork](/uniffi/README.md) and measures every wrapper
(Python, Go, Kotlin, Swift, Dart, C++, and the browser over ubrn) as moq's
`bench/` does today. Every fork perf quest states its effect with these
numbers. moq deletes its copy (`just bench-ffi`, `bench/ffi-gen`, and their
nightly jobs) in a paired PR.

## Plan

Decided with the maintainer on 2026-10-23: the custom FFI tooling and its
benchmarks leave moq, so the fork's perf work lives and measures here.
moq-ffi and its wrappers and packaging stay in moq; the benchmark drives them
at a pinned revision, as the [build-speed benchmark](/bench/build/README.md) pins
moq. A pin bump is its own change, so numbers compare across runs.

- Build the pinned moq-ffi against the fork through a local patch or flake
  override, so measuring a fork change never waits on moq's switch.
- Archive the BoltFFI comparison (`bench/ffi-gen`): keep its measured table in
  this benchmark's README as the target, and delete its harness. Git history
  holds it if a decision needs fresh numbers.
- Report variance with an A/A control and record the host, as the build-speed
  benchmark does.

Public API: none.
