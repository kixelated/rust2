# [S] Byte-slice locals that crash the translator

## Goal

rust2-ts never panics in `Fx::take_apart` (`ts/src/lower/mir/scalar.rs`): every value
`find_scalars` holds as parts can be taken apart, or `find_scalars` declines it, proven by a
fixture that crashed before the fix and by differential tests against native Rust.

## Plan

Decided with the maintainer on 2026-10-27: a rustc ICE report that
[#130](https://github.com/kixelated/rust2/pull/130) committed by accident records
`take_apart` reaching `unreachable!("checked by find_scalars")`. The input is lost, and
`main` still has the arm (`Shape::Enum`), so reproduce it first: likely an enum-shaped value
taken apart where `find_scalars` assumed none, from the codec, moq's model census, or a
fixture variant.

- Fix it where `find_scalars` and `take_apart` disagree, and check the file's other
  `checked by find_scalars` assumptions for the same gap.

Public API: none, unless the fix changes the translator's output.
