# [S] A tuple's parts read after their variables change

## Goal

rust2-ts translates a tuple built from variables and read after one of them is
reassigned with the value it held when built, so `(a, b) = (b, a)` on two `&[u8]` in a
loop swaps them instead of translating to `a = b; b = a;`, proven by a differential test
against native Rust.

## Plan

Found by [#130](https://github.com/kixelated/rust2/pull/130) on 2026-10-27: a swap in its
`splits` fixture gave 74 differences in about 344k cases, so the committed fixture leaves
the pattern out. The cause is on main: the `var_parts` arm of `place_expr` in
`ts/src/lower/mir/place.rs` reads a once-built tuple's field as the operand it was built
from, even when that operand is a variable reassigned before the read.

Decided with the maintainer on 2026-10-27: a silent miscompile, so it ranks first in m2.

- Fix it where the parts are recorded or read, for every operand kind, not only byte
  slices; check the other readers of `var_parts`.
- Put the swap back in the lang fixture, inside a loop.

Public API: the translator's output.
