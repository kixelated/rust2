# [M] Sessions on sans-IO QUIC

## Goal

A binding runs moq sessions over moq-quic's sans-IO QUIC with no runtime in the library:
the library owns QUIC and TLS, and the binding moves UDP datagrams and drives the session's
poll from its own event loop, as driving decided for the origin's
`Driver`. Proven from C in `moq/moq.c` against a local server.

## Plan

Decided with the maintainer on 2026-10-27: the full-control path, after
[sessions on moq-tokio](/quest/m1/ffi-export/session-tokio.md). It suits consumers with
their own event loop or sockets.

- Exported from moq-quic's and moq-net's `pub` API; where moq-quic's session does not yet
  fit moq-net's transport trait, that adapter is moq's change, not the exporter's.

Public API: the C names moq's bindings get.
