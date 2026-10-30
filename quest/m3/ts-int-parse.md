# [S] Integer parsing

## Goal

[rust2-ts](/ts/README.md) translates `str::parse` and `from_str_radix` of every integer
type, with `ParseIntError`'s kinds, `Display`, and `Debug`, proven by differential tests
against native Rust.

## Plan

Found by [#260](https://github.com/moq-dev/rust2/pull/260) on 2026-10-30, which mapped
`str::parse::<f64>`: no integer's `parse` has a mapping. moq-net and kio call none, so
this is m3 under the all-of-std decision.

- std's grammar, not `Number`'s or `parseInt`'s: an optional `+`, or `-` for a signed type,
  no whitespace or `_`, and `Empty`, `InvalidDigit`, `PosOverflow`, and `NegOverflow` as
  std picks them, including a lone sign.
- A 64- or 128-bit result is limbs past 2^53, so it can't go through a JS number.

Public API: the translator's output.
