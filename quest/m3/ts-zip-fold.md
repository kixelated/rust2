# [S] A zip of TrustedLen iterators folded by its size hint

## Goal

[rust2-ts](/ts/README.md) folds a `zip` of two `TrustedLen` iterators as std's `SpecFold`
does: `fold`, and what std folds through it (`count`, `last`, `for_each`, `sum`, and a
`TrustedLen` `collect`), reads as many pairs as its size hint's upper bound and no
further, so `r.by_ref().zip(0..n).count()` of a longer `RangeInclusive` leaves `r` read
`n` times, proven by a differential test against native Rust.

## Plan

Found by [#301](https://github.com/moq-dev/rust2/pull/301) on 2026-10-30. `rt.ZipIter`
reads `a` once past a shorter `b` in every fold, as std's does only by `try_fold` (`any`,
`find`, `position`) and for iterators that aren't `TrustedLen`. `rt.Iter.folded()` serves
both `fold` and `try_fold`, which agree for every other adaptor, so the zip needs a way to
tell them apart.

Public API: the translator's output.

## Related

- [Size hints of std's iterators](/quest/m3/ts-size-hints.md) - the upper bound std's fold reads
