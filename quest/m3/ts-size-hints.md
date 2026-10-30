# [M] Size hints of std's iterators

## Goal

[rust2-ts](/ts/README.md) translates `Iterator::size_hint` of std's iterators, and each
collect and extend of a collection that reserves by one, such as `Vec`'s `collect` and
`extend`, panics with std's `capacity overflow` where std's does, proven by differential
tests against native Rust.

## Plan

Found by [#284](https://github.com/moq-dev/rust2/pull/284) on 2026-10-30, which gives
`rt.Iter` a `lower` bound, exact for ranges and what adapts them, and reserves it in
`String`'s `Extend<char>`. `Vec`'s `collect` of a `TrustedLen` iterator reserves its upper
bound and panics without one, and its `extend` reserves the lower bound; `rt.iterToArray`
and `rt.iterExtend` reserve neither, so `(0..u32::MAX).map(f).collect::<Vec<_>>()` reads
on until memory runs out where std's panics before calling `f`. A call of `size_hint`
itself needs the upper bound too, and the crate's own `size_hint` is refused.

Public API: the translator's output.
