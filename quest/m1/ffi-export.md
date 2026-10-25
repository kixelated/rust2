# [XL] Exporter: moq's bindings from its crates' pub API

## Goal

[rust2-ffi](/ffi/README.md) replaces [the UniFFI fork](/uniffi/README.md) for moq: it exports
what moq's bindings need from moq's own crates, with no attributes and no hand-written wrapper
beyond what the census shows is genuinely new API, and generates the languages moq ships on its
C ABI. moq-ffi shrinks to that remainder. The fork stays in maintenance, building moq's bindings
until then.

## Plan

Decided with the maintainer on 2026-10-25, from the spike
([kixelated/rust2#41](https://github.com/kixelated/rust2/pull/41)): adopt the exporter, and fold
the fork's paused feature quests into it. Split into quests as work starts, ranked by what the
census refuses and what moq waits on:

- **Rules the census asks for**: tuples as out parameters (`origin::Producer::new`, so an
  origin can be made), `Path<'a>` exported as `Path<'static>`, `Range<u64>` and
  `RangeBounds`, and the borrowed `info()` returns of non-`Copy` objects, whose clone is a new
  handle.
- **Several crates**: moq-native's sessions and the media crates hold moq-ffi's remaining API.
  Export each with its own census, naming another export's types across crates, as
  moq-pattern's re-exports already need.
- **Driving**: sessions and origins run a `Driver` polled with a time and a `kio::Waiter`.
  Decide between exporting that poll, a generated thread that drives it, or moq-native's
  runtime helpers.
- **A contract symbol** per build, checked when bindings load, so a header and a library from
  different builds refuse to mix. It replaces the fork's namespaced contract and per-method
  checksums, which the export never had.
- **Generators** on the C ABI, measured on the [binding benchmark](/quest/m1/ffi-bench.md):
  - C++ per standard (17, 20, 23) over the one C ABI, compiled with GCC, Clang, and MSVC, with
    and without exceptions and RTTI; OBS builds at C17 and C++17.
  - Kotlin through JNI or Panama rather than JNA: the fork measured 3.2 us per call against
    7 ns for upstream's JNI bindgen.
  - Each keeps the C ABI's bulk and zero-copy payloads (no per-byte loop on either side) and
    reports its generated size, raw and gzipped, against the fork's (Python 653 KiB, Kotlin
    914 KiB, Swift 684 KiB for moq-ffi).
- **Async in each language**: the C ABI's futures already take the result in the poll that
  finishes them, so a cancel never loses a frame, cost three crossings when ready at once,
  and reuse a repeated waker. Each language's native future around them must latch a wake
  that lands between `PENDING` and parking, prove no loss on cancel in a test that forces that
  window, and pick a semantic for concurrent pollers. Go's cancel contract (moq's
  `go/wrapper/doc.go` ends the stream) is decided then. A hot read still makes a future handle
  per read; measure it before adding a per-handle poll.

## Related

- [Binding benchmark](/quest/m1/ffi-bench.md) - the fork's numbers, the baseline each generator is measured against
