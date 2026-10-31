# [S] Comparing iterators

## Goal

[rust2-ts](/ts/README.md) translates `Iterator::eq`, `ne`, `lt`, `le`, `gt`, `ge`, `cmp`,
and `partial_cmp`, and their `_by` forms, against any `IntoIterator`, ranges included,
reading each side as far as std's does, proven by differential tests against native Rust.

## Plan

Found while auditing what takes a `start..` as its `IntoIterator`: each refuses today
whatever the iterators are. Planned without a moq consumer, under the maintainer's
2026-10-27 decision that rust2 aims to support all of std.

- std's `eq` of two `TrustedLen` iterators decides by their size hints before reading
  any item where they differ; test it beside a pair that is not both `TrustedLen`.
- Test where a side ends first, where one never ends, and where comparing an item
  panics, as stepping a `start..` past its type's maximum does.

Public API: the translator's output.
