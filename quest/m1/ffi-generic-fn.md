# [M] Export instances of generic functions

## Goal

The [exporter](/ffi/README.md) exports a generic function or method at the instances the
config names, as it does generic types, so moq-ffi's flate producers and `with_publisher`
export from moq-mux and moq-net with no wrapper.

## Plan

Found by the generic-methods spike on 2026-10-27: moq's exported crates have 175 generic
functions; the rules export 109, and 47 are refused for a type parameter. The largest class
(19) is bounded by one of moq's own traits that exported Rust types implement, such as
`IntoRendition`, `Consume`, and `Container`. Binding generators refuse generic functions
(diplomat, cxx, interoptopus, wasm-bindgen, PyO3); swift-bridge and interoptopus
instantiate only generic types, by explicit lists. None of the four traits boxes to `dyn`
cleanly.

Decided with the maintainer on 2026-10-27:

- An `instances` key names a function or method and its type arguments by parameter name,
  each `impl Trait` argument included, such as `seeded(from: api_fixture::Point)`, and
  its C name, which is required as for type instances. Automatic instances at every
  exported implementor were left out: they grow the C API silently.
- A named instance replaces the rules for that function; any other stays refused as now.
- Refuse with a span an instance whose bounds fail (its help says to drop it from
  `instances`), that names nothing, or that nothing exported reaches.
- Name moq's instances, drop the excludes they make stale, and report each census and
  moq-ffi's coverage before and after.
- Out of scope until a consumer needs one: closures as callbacks lent for the call
  (`mutate`, beside `modify`'s `Guard`), `impl IntoIterator<Item = X>` as a `Vec` once
  `Vec` of records maps, `&T` in keys, and `serde_json::Value` for the `json_*`
  producers' `T`.

The spike's prototype (positional keys) is on local branch `spike/ffi-generic-methods`.

Public API: the exporter's config and the C names it emits.
