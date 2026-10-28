# [M] Setters for an object's pub fields

## Goal

A binding sets each `pub` field of an exported object, as Rust assigns it, so C can fill
in moq-tokio's session config: `connect::Config::url`, `tls.insecure`, `tls.fingerprint`,
a server's `listen.bind`, and `tls.generate`. Proven from C in `moq/moq.c`.

## Plan

Found from C on 2026-10-27 in [#161](https://github.com/kixelated/rust2/pull/161): a
`pub` field of an object exports only a getter returning a copy.

- An object is a handle to an `Arc`, so a setter needs a rule for a handle another
  handle shares; pick one that matches Rust's aliasing rather than hiding a copy.
- A nested field (`tls.insecure`) is a field of a field; decide whether the setter reaches
  through, or the binding gets, edits, and sets the whole value.
- Report the census before and after.

Public API: the C names the exporter emits.
