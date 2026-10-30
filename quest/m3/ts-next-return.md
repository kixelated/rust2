# [S] Return a std iterator's next from a generic function

## Goal

rust2-ts translates a generic function that returns `next` of a std iterator it was given,
`fn first<I: Iterator<Item = u8>>(mut it: I) -> Option<u8> { it.next() }` with `I` a
`Copied<slice::Iter<u8>>`, proven by a differential test against native Rust.

## Plan

Found while testing `&mut &mut I` on 2026-10-29: the translation is `return it.next();`,
the protocol's item or `rt.DONE` rather than the `Option` the signature declares, so `tsc`
refuses it. A `next` read into a local first, as `it.next().unwrap_or(0)`, converts.

Public API: the translator's output.
