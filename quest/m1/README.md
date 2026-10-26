# m1: consumers waiting

## Goal

The work moq is waiting on: the UniFFI fork its bindings switch to, and the exporter
that replaces it.

## Plan

Decided with the maintainer on 2026-10-25, from the
[spike](https://github.com/kixelated/rust2/pull/41): the [exporter](/ffi/README.md), which
turns a crate's `pub` API into bindings with no attributes, replaces the UniFFI fork. The
fork stays in maintenance and keeps building moq's bindings until then; its feature quests
are folded into the [exporter's](/quest/m1/ffi-export.md). The
[binding benchmark](/bench/ffi/README.md) continues as the exporter's baseline.

## Required

- [Waiting: publish the UniFFI fork](/quest/m1/uniffi-release.md) - the fork publishes to crates.io under `rust2-*` names, once the maintainer decides it should
- [Exporter](/quest/m1/ffi-export.md) - moq's bindings exported from its crates' `pub` API, replacing the fork
- [The exporter refuses what it can't export](/quest/m1/ffi-refuse.md) - `Option<&T>` parameters, unconstrained impl lifetimes, and unreachable objects work or are refused with a span
- [Measure moq's fixed crane in the Nix benchmark](/quest/m1/bench-nix.md) - crane after moq's layering fixes beside the per-crate caches, so moq can choose how its Nix builds share compilations
