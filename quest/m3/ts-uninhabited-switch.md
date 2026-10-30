# [S] A switch whose other variants are uninhabited

## Goal

[rust2-ts](/ts/README.md) translates a match or `let` that tests a part of a tuple or
other non-path place whose other variants can't exist, such as
`let (Ok(a), Ok(b)) = (wide, wider);` of two `Result<_, Infallible>`s, without leaving the
discriminant it read unused, so `tsc` accepts it, proven by differential tests against
native Rust.

## Plan

Found by the quest that typed an infallible `try_from` by its `Infallible`, on 2026-10-30:
MIR reads each part's discriminant and switches to `unreachable` otherwise; lowering drops
that one-way switch, but the read was already spilled to a `const scrutinee` (the
`Slot::Discr` assignment in `ts/src/lower/mir/mod.rs`), which `noUnusedLocals` refuses.
A single `match` or `let Ok(x) = r` of such a `Result` already type-checks. No consumer
hits it at the pin.

Public API: the translator's output.

## Related

- [A parameter read only where lowering leaves nothing](/quest/m3/ts-unread-params.md) - the same kind of read lowering leaves unused, of a parameter
