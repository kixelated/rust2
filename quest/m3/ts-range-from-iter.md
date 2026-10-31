# [S] Unbounded ranges as iterators adaptors take

## Goal

An unbounded `start..` that an adaptor takes as its `IntoIterator`, as `iter.zip(0..)`
does, translates to an iterator `tsc` accepts, proven by a differential test against
native Rust, including where `start` reaches its type's maximum.

## Plan

Found by `keys::arrays` in the lang fixture: `firsts.iter().zip(0..)` translates to
`rt.iterZip(rt.iterSlice(firsts), { start: 0 })`, which `tsc` rejects, since the range's
struct reaches `iterZip` in place of an `rt.Iter`. Audit every adaptor that takes an
`IntoIterator` (`zip`, `chain`, `extend`, `eq`, `cmp`) for the same.

Public API: the translator's output.
