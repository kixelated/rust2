# [S] Returned iterators cross as arrays

## Goal

moq-net's `Hops::iter` and `Versions::iter` and moq-pattern's `Patterns::iter` export
without being excluded in `ffi/moq/ffi.toml`, each returning what it iterates as an array,
as a `&[T]` return crosses. This comes from a fixed rule, with no config.

## Plan

Found by [#205](https://github.com/kixelated/rust2/pull/205) on 2026-10-28, which exports
`&[T]` of any type.

- `std::slice::Iter<'_, T>` is a borrowed slice, so it crosses exactly as `&[T]` does.
- `Versions::iter` returns `impl Iterator<Item = &Version>`. Collecting it at the return is
  exact only for an iterator that reads borrowed data, with no side effect and an end;
  decide which `impl Iterator` returns the rule takes when the quest starts, and refuse the
  rest with a span.
- Report moq-ffi's coverage and the census before and after.

Public API: the C names and types these methods export with.
