# [S] Strings from UTF-8 bytes

## Goal

[rust2-ts](/ts/README.md) translates `std::str::from_utf8` and `Utf8Error` (its
`valid_up_to`, `error_len`, `Display`, and `Debug`), with std's results on invalid input,
proven by differential tests against native Rust. `just ts moq` passes at the moq revision
past [moq-dev/moq#6843](https://github.com/moq-dev/moq/pull/6843), whose decoder calls it.

## Plan

Decided with the maintainer on 2026-10-27: the exporter and the translator share one moq
pin, so [the exporter's pin bump](https://github.com/kixelated/rust2/pull/144) merges once
this passes at its revision.

- `TextDecoder` with `fatal` rejects what std rejects, but the error position must be std's
  `valid_up_to` and `error_len`, so check its edge cases (truncated sequences, surrogates,
  overlongs) against native.

Public API: the translator's output.
