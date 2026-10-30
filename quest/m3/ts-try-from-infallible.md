# [XS] A widening try_from typed by its Infallible

## Goal

[rust2-ts](/ts/README.md) types `u16::try_from(u8)` and every other integer `try_from` or
`try_into` that can't fail as std does, `Result<u16, Infallible>`, so it type-checks where a
value of that type is named, proven by differential tests against native Rust.

## Plan

Found by the quest that made `Result` and std's errors comparable, on 2026-10-30: the mapping
returns `rt.u16TryFrom`'s `Result<u16, TryFromIntError>` whatever the conversion, and `tsc`
refuses it where the Rust type is `Result<u16, Infallible>`, typed `Result<u16, never>`.

- Where `From` holds, the conversion is `rt.Ok` of the widened value.

Public API: the translator's output.
