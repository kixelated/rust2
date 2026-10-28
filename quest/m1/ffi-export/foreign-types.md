# [M] Session config's foreign types

## Goal

The exporter maps the types moq-tokio's session config and calls take that it refuses
today: `url::Url` (`connect::Config::url`, `Client::connect`), `std::net::SocketAddr`
(`connect::Config::bind`, `Server::local_addr`), `PathBuf` (TLS files), and `Vec<String>`
(`tls.fingerprint`, `tls.generate`). Proven from C in `moq/moq.c`, including a test that
binds port 0 and reads the address back.

## Plan

Found from C on 2026-10-27 in [#161](https://github.com/kixelated/rust2/pull/161).

- These are general rules, not moq's: each maps wherever it appears, with no config.
- `Url` and `SocketAddr` could cross as text that Rust parses, failing loud with the
  parse error, or as a type of their own; prefer what every generator can read.
- `PathBuf` holds OS bytes, not UTF-8; don't lose the ones a `String` can't hold.
- `Vec<String>` may be the first `Vec` of a non-byte type; share its shape with the
  `Vec` that [instance keys](/quest/m1/ffi-export/instance-keys.md) need.
- Report the census before and after.

Public API: the C names the exporter emits.
