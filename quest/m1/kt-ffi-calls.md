# [M] Kotlin binding calls without JNA overhead

## Goal

A Kotlin call into moq-ffi costs about what the same call costs from Python or
Go, sync and async alike, proven on the
[binding benchmark](/quest/m1/ffi-bench.md).

## Plan

Measured 2026-10-15 in moq: a sync `sequence()` took 11-22 us from Kotlin
against 1.3 us from Python and 0.12 us from Go, and the 100 B throughput case
kept about 5.8 cores busy for 2 MB/s.

Decided with the maintainer on 2026-10-15 in moq, and moved here on 2026-10-23:

- **Profile first** (JFR or async-profiler) and confirm each cause before
  changing anything. The two suspects in the generated Kotlin:
  - Every call allocates a JNA `Structure` (`UniffiRustCallStatus`, which
    nests a `RustBuffer.ByValue`), and JNA builds and syncs those by
    reflection
    ([mozilla/uniffi-rs#3011](https://github.com/mozilla/uniffi-rs/issues/3011)).
  - The async continuation is a plain JNA `Callback` with no
    `CallbackThreadInitializer`, so each wakeup from a Rust worker attaches
    that thread to the JVM and detaches it again.
- **Fix in the fork's Kotlin generator**, not by patching generated code. Both
  costs are one quest: same root, same generator, same proof. Prefer a shape
  that keeps the wrapper's API unchanged.
- The BoltFFI spike measured the target: its JNI backend makes a no-op call in
  6 ns and a 100 B `write_frame` in 134 ns, against UniFFI's 3.2 and 12.8 us.

Decided with the maintainer on 2026-10-23: try cherry-picking upstream's
experimental Kotlin JNI bindgen
([mozilla/uniffi-rs#2911](https://github.com/mozilla/uniffi-rs/pull/2911), on
upstream `main` after 0.32.2, unreleased) before hand-fixing JNA. Its author
measured a call at 3.04 us to 7 ns and bytes at 14.7 us to 87 ns
([gist](https://gist.github.com/bendk/7bc8886cb33b8bd5d257336dc33d54a5)),
about even with BoltFFI. List it as a carried change in the fork's README.

Public API: none expected.

## Required

- [Import UniFFI](/quest/m1/uniffi.md) - the fork whose generator this patches
- [Binding benchmark](/quest/m1/ffi-bench.md) - the before and after numbers
