# [XS] Zip's next_back in constant time

## Goal

`next_back` of a `Zip` of two `TrustedRandomAccess` iterators, such as ranges or slices,
skips the longer side's excess in constant time as std's does, rather than reading each
item, so a zip of `0..u32::MAX` with `0..1` reversed returns at once. Benchmarked.

## Plan

`rt.TrustedZipIter` inherits `ZipIter`'s trim, which calls `nextBack` once per excess item.
std reads no excess item of a side whose `MAY_HAVE_SIDE_EFFECT` is false and indexes by
position; a side that may have side effects still runs them.

Public API: the translator's output.
