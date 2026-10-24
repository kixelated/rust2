# [S] Namespaced FFI contract in the UniFFI fork

## Goal

A library built with [the fork](/uniffi/README.md) and bindings from upstream
UniFFI, or the reverse, fail loudly when the bindings load instead of mixing
silently. Lands with or before the fork's first change to the FFI contract
(metadata, checksums, or scaffolding signatures).

## Plan

The import kept upstream 0.32's contract (version 30) and every symbol name so
moq's generated bindings stayed byte-identical. A namespace necessarily changes
every generated binding, so it waits for the first change that changes them
anyway.

- Prefer a fork-specific contract symbol (for example
  `ffi_<namespace>_rust2_uniffi_contract_version`) over a new version number,
  which can collide with an upstream bump. Every generator (Kotlin, Swift,
  Python, Go, Dart, C++, and C) must check it.
- Prove it with a fixture: each generator's bindings refuse a cdylib built
  with upstream UniFFI.
- [Drop checksums](/quest/m2/ffi-checksums.md) is the first consumer: the
  namespace is what refuses a mismatched library once the per-method
  checksums are gone.

Public API: the generated contract check, and the scaffolding's exported
contract symbol.
