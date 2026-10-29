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
- Decided with the maintainer on 2026-10-28, replacing an earlier plan to cross JSON as
  text: `serde_json::Value` is a runtime JSON object, as `url::Url` is, never a string. It
  has only a parse from JSON text, whose error is the only place a parse fails, and a
  `to_text`; callers read its contents with their own JSON library. Every function keeps
  its Rust signature, taking or returning the handle, so moq needs no change. A serde
  `Serialize` or `Deserialize` bound infers `serde_json::Value`, not `String`.
- `json::Snapshot::update` and `json::Stream::append` take `impl Into<Timed<&'a T>>`,
  which needs [#197](https://github.com/kixelated/rust2/pull/197)'s `From`-impl rule
  extended to a concrete source (`From<&'a V> for Timed<&'a V>` gives `&T`). Then moq's
  tests publish a value on the JSON track and read it back.
- Report moq-ffi's coverage and the census before and after.

Public API: the C names and types these methods export with.
