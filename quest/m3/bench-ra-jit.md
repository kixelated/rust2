# [S] ra-jit in the build benchmark

## Goal

The [build-speed benchmark](/bench/build/README.md) measures
[ra-jit](https://github.com/indexable-inc/ra-jit) as a variant on the moq-net edit
scenario, and records how much of moq-net it refuses or hands back to cargo.

## Plan

Decided with the maintainer on 2026-10-24, from build-speed research. ra-jit is a
daemon on a forked rust-analyzer that compiles only the changed functions through
Cranelift and patches them into the already-linked binary, so an edit has no SVH
cascade and no relink. It claims a 12-24x geometric mean, such as a nushell edit
going from 14.4 s to 0.85 s.

- Its `outline.rs` treats an `async fn` body as part of the signature, and moq-net
  is mostly async, so record the share it hands back to cargo beside its times.
- It needs Rust 1.99.0 and several GB of daemon memory, so it runs as its own
  toolchain variant.

Public API: a benchmark variant.

## Related

- [Early cutoff for edit rebuilds](/quest/m3/edit-cutoff.md) - the rustc-side route past dependent rebuilds
- [Relink without rebuilding](/quest/m3/relink.md) - upstream's route past the same rebuilds
