# Exporter: moq's bindings from its crates' pub API

## Goal

[rust2-ffi](/ffi/README.md) replaces [the UniFFI fork](/uniffi/README.md) for moq: it exports
what moq's bindings need from moq's own crates, with no attributes and no hand-written wrapper
beyond what the census shows is genuinely new API, and generates the languages moq ships on its
C ABI. moq-ffi shrinks to that remainder. The fork stays in maintenance, building moq's bindings
until then.

## Plan

Decided with the maintainer on 2026-10-25, from the spike
([kixelated/rust2#41](https://github.com/kixelated/rust2/pull/41)): adopt the exporter, and fold
the fork's paused feature quests into it.

Decided with the maintainer on 2026-10-27: this is an epic, with each item a child ranked by
what the census refuses and what moq waits on first, then the runtime, then the generators.
What stays here is the end-to-end: moq-ffi shrinks to its remainder and takes the stand-in
package's place.

Every generator on the C ABI:

- checks `<prefix>_contract()` against the header's contract when it loads, before any
  other call, and fails loud on a mismatch;
- is measured on the [binding benchmark](/bench/ffi/README.md);
- keeps the C ABI's bulk and zero-copy payloads, with no per-byte loop on either side;
- reports its generated size, raw and gzipped, against the fork's (Python 653 KiB, Kotlin
  914 KiB, Swift 684 KiB for moq-ffi);
- wraps the C ABI's futures in the language's native future. Those futures already take the
  result in the poll that finishes them, so a cancel never loses a frame, cost three crossings
  when ready at once, and reuse a repeated waker. The native future must latch a wake that
  lands between `PENDING` and parking, prove no loss on cancel in a test that forces that
  window, and pick a semantic for concurrent pollers. A hot read still makes a future handle
  per read; measure it before adding a per-handle poll. Decided with the maintainer on
  2026-10-27: async lands with each generator, not as its own quest.
- drives each of moq's `Driver`s from the language's own event loop. Decided with the
  maintainer on 2026-10-27: the C ABI exports the poll, with no generated thread and no
  moq-tokio helpers, which would tie bindings to tokio. A binding holds one `kio::Waiter`
  per driver, made from its waker, since what a poll registers lives only as long as its
  waiter; it polls on each wake and at the deadline the poll returns, and frees the driver
  to cancel it. Decided with the maintainer on 2026-10-27, for Go: a cancel ends only that
  read and leaves the stream usable, as in Rust, since a cancelled C ABI future loses
  nothing; moq's `go/wrapper/doc.go`, which ends the stream, changes with the generator.
  Each driver runs on a goroutine that lives as long as its object, and a call's `ctx`
  bounds only that call.

## Required

- [Instances the config can't name yet](/quest/m1/ffi-export/instance-keys.md) - `&T` keys, `IntoIterator` as a `Vec`, `serde_json::Value`, and `Catalog<()>`
- [moq-clock's C names once it moves into kio](/quest/m1/ffi-export/crates.md) - `moq_now` and `moq_origin` become the clock's, after moq moves it into `kio::clock`
- [Setters for an object's pub fields](/quest/m1/ffi-export/field-setters.md) - C fills in a session's config, as Rust assigns it
- [Session config's foreign types](/quest/m1/ffi-export/foreign-types.md) - `Url`, `SocketAddr`, `PathBuf`, and `Vec<String>` cross the C ABI
- [Sessions on moq-tokio](/quest/m1/ffi-export/session-tokio.md) - bindings connect and accept with one call, the library running moq-tokio inside
- [Sessions on sans-IO QUIC](/quest/m1/ffi-export/session-quic.md) - bindings move UDP datagrams and drive the session's poll themselves
- [C++ bindings on MSVC and Clang](/quest/m1/ffi-export/cpp.md) - the C++ header built and tested beyond GCC on Linux
- [Kotlin bindings: zero-copy payloads and the read path](/quest/m1/ffi-export/kotlin.md) - the Kotlin lends bytes without a copy, and reads at JNI's cost
