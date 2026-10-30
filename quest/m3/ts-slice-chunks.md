# [S] The rest of a slice's chunks

## Goal

rust2-ts translates a slice's `chunks_exact`, `rchunks`, `rchunks_exact`, and their
`_mut` forms, and `chunks_mut`, matching std's parts, remainders, and panics, and writing
through a mutable chunk into the slice it came from. Proven by differential tests against
native Rust.

## Plan

Each refuses with a span today. `windows` and `chunks` map to `rt.PartsIter` in
`rt/iter.ts` ([#259](https://github.com/moq-dev/rust2/pull/259)), which the others can
extend. A mutable chunk of a slice that is not bytes is a JS array copied by `slice`, so
writes through it would be lost: it needs a view or a place, as `iter_mut` lends one.
rust2 aims to support all of std.

Public API: the translator's output.
