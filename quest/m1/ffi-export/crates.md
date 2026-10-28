# [M] One library from several crates

## Goal

moq-net, moq-pattern, moq-clock, moq-tokio, hang, and moq-mux export into one library
with every C name the bindings need, so nothing is excluded for a name clash.

## Plan

They export through an empty package standing in for moq-ffi's remainder, which
`just ffi standin` writes. Left:

- moq-clock's C names (`moq_now`, `moq_origin`) wait for its merge into kio.
- A field and a method of one name, such as moq-tokio's `Backoff::initial`, claim one C
  name that `rename` cannot split, so [the config](/ffi/moq/ffi.toml) excludes both.

Public API: the exporter's config and the C names it emits.
