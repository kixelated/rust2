# [M] Sessions on moq-tokio

## Goal

A binding connects and accepts moq sessions with one call, as moq-ffi does today: the
exported library runs moq-tokio's sockets and runtime inside, and the binding never drives
a transport. Proven from C in `moq/moq.c` against a local relay or server.

## Plan

Decided with the maintainer on 2026-10-27, from
[#150](https://github.com/kixelated/rust2/pull/150): `Client::connect` and `Server::accept`
need a transport C can't supply. Bindings get both: this easy path first, since most
consumers want it, then [the sans-IO path](/quest/m1/ffi-export/session-quic.md).

- The runtime inside is moq-tokio's own, exported from its `pub` API like everything else.
  Where its API needs a wrapper to cross the C ABI, that wrapper is new API in moq-tokio.

The one call already exports: `Client::publish` and `consume` dial `connect.url`, and
`Server::serve_publish`, `serve_consume`, and `serve_both` accept until the listener stops.
Found from C on 2026-10-27 at the pinned moq: `connect::Config::init` returns moq-tokio's
`NoRuntime`, since its QUIC endpoint needs the current tokio runtime, and
`Client::connect` would `tokio::spawn`, a panic that aborts. The futures C polls
(`Server::bind`, the `serve_*`, `Listener::accept`, `Request::ok`) make timers and spawn in
their bodies, so each poll needs the runtime entered, not only the call.

Decided with the maintainer on 2026-10-28: moq-tokio starts a fallback runtime on first
use unless the caller supplies one. Each entry point and future that needs tokio runs in
the caller's `Handle::try_current()`, else in that process-wide runtime. C gets no new
names, a Rust caller inside tokio sees no change, and one outside gets a runtime instead
of `NoRuntime`. That lands in moq, then a pin bump here.

Public API: the C names moq's bindings get.

## Required

- [Setters for an object's pub fields](/quest/m1/ffi-export/field-setters.md) - C fills in the session config
- [Session config's foreign types](/quest/m1/ffi-export/foreign-types.md) - `Url`, `SocketAddr`, `PathBuf`, and `Vec<String>` cross the C ABI

