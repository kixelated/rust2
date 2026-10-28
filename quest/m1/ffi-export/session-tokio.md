# [L] Sessions on moq-tokio

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
Found on 2026-10-27 at the pinned moq, from C, what keeps C from calling them:

- **No runtime.** `connect::Config::init` returns moq-tokio's `NoRuntime` ("no async
  runtime"), since its QUIC endpoint needs the current tokio runtime, and
  `Client::connect` would `tokio::spawn`, a panic that aborts. The futures C polls
  (`Server::bind`, the `serve_*`, `Listener::accept`, `Request::ok`) make timers and spawn
  in their bodies, so each poll needs the runtime entered, not only the call.
- **No config.** A `pub` field of an object exports only a getter, so C can't set
  `connect::Config::url`, its `tls.insecure` or `tls.fingerprint`, a server's `listen.bind`,
  or `tls.generate`. Rust assigns a `pub` field, so the rule can export a setter too.
- **Foreign types.** Refused: `url::Url` (`connect::Config::url`, `Client::connect`),
  `std::net::SocketAddr` (`connect::Config::bind`, and `Server::local_addr`, which a test
  binding port 0 reads), `PathBuf` (TLS files), and `Vec<String>` (`tls.fingerprint`,
  `tls.generate`).

The config and foreign types land without the runtime, so they can split into quests of
their own ahead of this one.

Open: how moq-tokio supplies its runtime.

1. Recommended: moq-tokio enters a runtime of its own when the caller is in none. Each
   entry point and future that needs tokio runs in `Handle::try_current()`, else in a
   process-wide runtime moq-tokio starts on first use. C gets no new names, a Rust caller
   inside tokio sees no change, and one outside gets a runtime instead of `NoRuntime`.
2. An explicit `moq_tokio::Runtime` that `init` takes and each object keeps, entering it
   to spawn and on each poll. It breaks every Rust caller of `init`.
3. Wrappers on a `moq_tokio::Runtime` for the easy path only (`init`, `publish`,
   `serve_*`), each spawned on it. They duplicate API C already has.

Public API: the C names moq's bindings get.
