# [M] Export instances of generic types

## Goal

The [exporter](/ffi/README.md) exports a generic type at the instances the config names,
such as `catalog::Producer<hang::Extra>`, so moq-ffi's media API that uses each at one
type exports from moq-mux and hang.

## Plan

Found by [#94](https://github.com/kixelated/rust2/pull/94) on 2026-10-27: refused generic
types block about 67 of moq-ffi's partly covered items, mostly in moq-mux
(`catalog::Producer<T>`, `container::Consumer<T>`). moq-ffi uses each at one type.

- Name each instance in [the config](/ffi/moq/ffi.toml), with the C name it exports
  under, and refuse with a span any signature that reaches a generic type at an instance
  the config does not name.
- Report each export's census and the coverage of moq-ffi's API before and after.

Public API: the exporter's config and the C names it emits.
