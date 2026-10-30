# [S] Equality of Result and std's errors

## Goal

[rust2-ts](/ts/README.md) translates `==` and `!=` of a `Result` and of std's error types,
`ParseIntError`, `ParseBoolError`, `ParseCharError`, `ParseFloatError`, and
`TryFromIntError`, as `assert_eq!(s.parse::<bool>(), Ok(true))` uses them, proven by
differential tests against native Rust.

## Plan

Found by [#278](https://github.com/moq-dev/rust2/pull/278) on 2026-10-30, whose `str::parse`
of `bool`, `char`, and `NonZero` each refused `<Result<_, _> as PartialEq>::eq`. moq-net and
kio compare none of these, so this is m3 under the all-of-std decision.

- `ParseCharError`'s kinds are private, so `==` is the one way besides its `Display` and
  `Debug` to tell them apart: test both kinds against each other.

Public API: the translator's output.
