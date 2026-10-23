# rust2 questline

## Goal

Keep the repository's living work organized as visible, versioned quests,
grouped into milestones ordered by priority.

## Plan

m0 holds urgent fixes. m1 is the work with a consumer waiting on it now:
the UniFFI fork moq switches to. m2 is the TypeScript translator, a research
track moq compares against its WASM route. m3 holds the subset tracks, which
the [build-speed benchmark](/bench/README.md) rescopes before they start, and
speculative work.

Decided with the maintainer on 2026-10-23, when rust2 was planned from moq
([moq-dev/moq#6578](https://github.com/moq-dev/moq/pull/6578)):

- One repository with three projects and one setup, copied from moq's reusable
  parts: the nix flake and direnv, just, the quest tree, agent docs and hooks,
  and the shared skills submodule.
- moq is the first consumer of each project, and pins rust2 as a flake input.
  moq's browser line still ships WASM first; the translator here does not
  replace that plan.
- The first quest that adds a crate also adds the Cargo workspace (moq's
  workspace lints, including a reason on every suppression) and the cargo
  steps of `just check`.

## Required

- [m0: immediate priorities](/quest/m0/README.md) - urgent fixes
- [m1: consumers waiting](/quest/m1/README.md) - the UniFFI fork moq switches to
- [m2: TypeScript](/quest/m2/README.md) - an own rustc driver translates moq-net's codec, and the fork's namespaced types
- [m3: deferred](/quest/m3/README.md) - the pure-Rust profile and subset compiler, rescoped by the benchmark
