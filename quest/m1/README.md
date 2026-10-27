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
- [Export instances of generic types](/quest/m1/ffi-generic.md) - a generic type exports at the instances the config names
- [Land main through quest land](/quest/m1/land.md) - every queued PR lands as one tested batch through the shared gate
- [Bump quest to quest land](/quest/m1/land-bump.md) - condition: kixelated/quest ships `quest land`; then bump the flake input
