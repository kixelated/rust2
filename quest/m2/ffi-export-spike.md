# [L] Exporter spike: a crate's pub API to C

## Goal

A prototype exporter turns a crate's `pub` API into a C ABI and a C header, with no
attributes in the Rust source. It runs on moq-net's public API directly, not moq-ffi,
and its report counts every refusal, which shows how much of the moq-ffi wrapper crate
is really needed. It compares the C output with moq's hand-written `rs/moq-c`, and
decides whether the exporter replaces [the UniFFI fork](/uniffi/README.md) and what
becomes of the fork feature quests paused behind it.

## Plan

Decided with the maintainer on 2026-10-24: replace UniFFI's macros with an opinionated
Rust-to-language exporter. A crate's `pub` API alone defines what gets exported, with no
`#[uniffi::export]` attributes.

- **Source**: a rustc driver, reusing [rust2-ts](/ts/README.md)'s driver infrastructure
  on the same pinned nightly, so types, visibility, and trait impls come from the
  compiler. Chosen over rustdoc JSON and parsing the source.
- **Output**: its own IR and its own generators, not UniFFI metadata.
- **Opinions**: every `pub` item reachable from the crate root is exported by fixed
  rules:
  - a struct whose fields are all `pub` becomes a record, and other types become
    objects behind `Arc`;
  - `Result` becomes errors;
  - `async fn` becomes a future;
  - `pub` traits become callback interfaces.

  Anything that doesn't map is a compile error with a source span, unless a config file
  excludes it. Nothing is skipped silently, and nothing is opted in by attribute.
- **Target**: moq-net's public API, at a pinned moq revision as `just ts moq` pins one.
  The refusal count is the measure of what a wrapper crate still has to add.
- **First output**: a C ABI plus a C header, matching the earlier C-first decision:
  every other language layers on that one ABI later. Compare it against moq's
  hand-written `rs/moq-c`.
- **The UniFFI fork**: spike first, with the fork in maintenance. New fork features
  pause behind this spike: the C-first bindings spike
  ([kixelated/rust2#34](https://github.com/kixelated/rust2/pull/34) adds it), the
  [namespaced contract](/quest/m2/uniffi-contract.md), and the perf quests
  ([payload bytes](/quest/m1/ffi-bytes.md),
  [zero-copy returns](/quest/m2/ffi-bytes-return.md),
  [cheaper async](/quest/m2/ffi-async.md),
  [checksums](/quest/m2/ffi-checksums.md),
  [code size](/quest/m2/ffi-code-size.md),
  [Kotlin calls](/quest/m1/kt-ffi-calls.md), and
  [foreign poll](/quest/m1/ffi-poll-protocol.md)). Their findings feed the exporter's
  design, and the report says, for each, whether it is folded into the exporter. The
  fork keeps building moq's bindings until the exporter replaces it.

Public API: a new tool.

## Related

- [Binding benchmark](/quest/m1/ffi-bench.md) - continues, as the baseline the exporter is measured against
