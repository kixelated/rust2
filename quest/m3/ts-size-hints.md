# [M] Size hints of std's iterators

## Goal

[rust2-ts](/ts/README.md) translates `Iterator::size_hint` of std's iterators, and each
collect and extend that reserves by one, such as `String`'s `Extend<char>` and `Vec`'s
`collect`, panics with std's `capacity overflow` where std's does, proven by differential
tests against native Rust.

## Plan

Found by [#284](https://github.com/moq-dev/rust2/pull/284) on 2026-10-30, which
translates `String`'s `extend` and `collect` by `for_each` but skips the `reserve` of
the lower bound std makes first. An iterator whose lower bound passes `isize::MAX` on
wasm32, such as `repeat(c)` or `(0..u32::MAX).map(f)`, panics there in std before it
yields an item, where the translation reads on until memory runs out. `rt.Iter` keeps no
size hint, and the crate's own `size_hint` is refused, so no translated code observes one
today.

Public API: the translator's output.
