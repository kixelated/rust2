# [M] Iterator and JSON arguments the exporter can't map yet

## Goal

moq-net's `track::Subscription::with_ranges(impl IntoIterator<Item = Range>)` exports,
taking a `Vec` of `Range`, and moq-mux's `catalog::Producer::json_snapshot` and
`json_stream` export at `serde_json::Value` as their `T`, crossing as something every
generator can read. Both come from fixed rules, with no config: moq's crates export as
they are.

## Plan

Decided with the maintainer on 2026-10-27, superseding this quest's plan to extend the
config's `instances` key: translators take stock crates as they are, so the exporter
infers the instances it exports at, and the `instances` key is gone
([#152](https://github.com/kixelated/rust2/pull/152)). That PR also exports this quest's
other two items: moq-net's `with_publisher` at `&origin::Producer`, and `DEFAULT_NAME` on
hang's `Catalog<()>`. What remains is two type mappings.

- `impl IntoIterator<Item = X>` takes a `Vec<X>`, which moq-net's `with_ranges` exports
  through.
- Decided with the maintainer on 2026-10-28: `serde_json::Value` crosses as JSON text, a
  UTF-8 document string in every language, and callers use their own JSON library. A
  `T: Serialize` parameter infers `serde_json::Value`, not `String`, so moq's config stops
  excluding `json_snapshot` and `json_stream`. Malformed text from a caller is an error
  result, never an abort.
- Open: where that error goes. `json::Snapshot::update` and `json::Stream::append` return
  moq-mux's `Error`, which has no `From<serde_json::Error>`, so a parse failure has no
  home in the call's own result.
- Report moq-ffi's coverage and the census before and after.

Public API: the C names and types these methods export with.
