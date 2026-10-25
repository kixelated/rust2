# [XS] Run the build benchmark on PRs only when they change what it measures

## Goal

A PR runs the [build-speed benchmark](/bench/build/README.md) only when it
changes what the benchmark measures: its scripts and corpus, the
`build-bench` shell's toolchains and tools, or the workflow itself. A
`flake.nix` or `flake.lock` change that leaves the `build-bench` shell alone,
or a docs-only change under `bench/build/`, runs nothing. Each run costs
about 2.7 runner-hours of CPU, and the maintainer weighs CPU time over wall
clock.

## Plan

[bench.yml](/.github/workflows/bench.yml)'s `paths` filter already skips
`bench/ffi/`. It still fires on any `flake.nix` or `flake.lock` change,
which most PRs touching the dev shell make. Path filters cannot tell which
shell a flake change touches, so consider comparing the `build-bench`
shell's derivation between base and head in a cheap gate job, and run the
matrix only when it differs. Check that the gate does not cost more than
it saves, and that the nightly schedule and `workflow_dispatch` still run
everything.

Public API: none; CI only.
