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

Public API: the C names moq's bindings get.
