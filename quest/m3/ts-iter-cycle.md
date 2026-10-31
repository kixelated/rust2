# [S] Cycling std's iterators

## Goal

`Iterator::cycle` translates, proven by a differential test against native Rust, which
the reject fixture's `cycled` refuses today.

## Plan

Found by [#322](https://github.com/moq-dev/rust2/pull/322), which gave each `rt.Iter`
that std implements `Clone` for a `clone(h)` and the translation its `rt.Clones`. std's
`Cycle` holds its iterator as it was and a clone it reads, which it clones again each
time that ends, so an adaptor of the runtime's can hold the original and its `Clones`
and clone it the same way. Planned without a moq consumer, under the maintainer's
2026-10-27 decision that rust2 aims to support all of std.

Public API: the translator's output.
