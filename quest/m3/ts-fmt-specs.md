# [M] The rest of format specs

## Goal

rust2-ts translates the format specs it refuses today, byte for byte as `core::fmt`
writes them, proven by differential tests against native Rust: a precision (`{:.3}`,
`{:.N?}`, `{:.*}`), widths and precisions from arguments (`{:1$}`, `{:w$}`), `+`, `{:b}`,
`{:o}`, `{:e}`, and `{:x?}`/`{:X?}`.

## Plan

Decided with the maintainer on 2026-10-27: rust2 aims to support all of std, so generic
gaps are planned without a moq consumer. Found by
[#125](https://github.com/kixelated/rust2/pull/125): `{:.N?}` is the usual way to print a
`Duration`.

- The runtime ports `core::fmt` ([output](/ts/README.md)), so each spec belongs in that
  port rather than a per-type special case. Each type's `Display` and `Debug` treat a
  precision differently (floats round, strings truncate, `Duration` rounds its fraction),
  so cover each.
- Update the README's list of refused specs.

Public API: the translator's output.
