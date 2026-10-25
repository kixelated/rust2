# [S] Cranelift on moq's CI test builds

## Goal

The [build-speed benchmark](/bench/build/README.md) answers whether moq's CI
should build its tests with `-Zcodegen-backend=cranelift`: the CPU and wall
time it saves on building and running moq's tests the way CI does, from clean
and after an edit, and whether every test passes. The README records the
result and a recommendation.

## Plan

Decided with the maintainer on 2026-10-25: trial Cranelift. The nightly
benchmark already measures it on `cargo build` (16% less CPU from clean) and
`cargo check`, but nothing measures a test build, nor the test run, where
Cranelift's slower code could cost back what the build saves.

- moq's CI builds and runs its tests with `cargo nextest run --tests` on
  dependencies its compilation cache serves, so a test build after an edit is
  closer to CI than a clean one; time both, and the run on its own.
- Keep the nightly cost small: only the nightly base, Cranelift, and their
  control need the test scenarios, and only on moq.

Public API: benchmark scenarios.
