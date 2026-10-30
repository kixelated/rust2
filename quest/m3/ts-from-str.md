# [S] Other FromStr types

## Goal

[rust2-ts](/ts/README.md) translates `str::parse` of `bool`, `char`, and `NonZero`, with
`ParseBoolError` and `ParseCharError`, proven by differential tests against native Rust.

## Plan

Found by [#266](https://github.com/moq-dev/rust2/pull/266) on 2026-10-30, which parses every
integer by core's own `from_str_radix`. moq-net and kio call none of these, so this is m3
under the all-of-std decision.

- `str::parse` and each `FromStr` lower from core's MIR where they can, as the integers'
  do; `NonZero`'s calls theirs, adding `IntErrorKind::Zero`.
- `ParseCharError`'s kinds are private, so only its `Display` and `Debug` show them.
- `f32` parses with [f32](/quest/m3/ts-f32.md).

Public API: the translator's output.
