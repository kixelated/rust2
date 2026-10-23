# [L] Binding payloads cross the FFI without per-byte copies

## Goal

A binding moves a frame payload across the FFI with no per-byte loop: every
copy on the path is a bulk copy, and the report counts them per direction.
Proven on the [binding benchmark](/quest/m1/ffi-bench.md) in every wrapper
(Python, Go, Kotlin, Swift, Dart, C++, and the browser).

## Plan

Decided with the maintainer on 2026-10-13 in moq, and moved here on 2026-10-23
with the binding benchmark:

- **The browser too.** Since 2026-10-17
  ([moq-dev/moq#5838](https://github.com/moq-dev/moq/pull/5838)) the browser
  binds moq-ffi through UniFFI over ubrn, and pays the same per-byte path: about
  7 ns per byte written and 8 ns read at 128 KiB in Chromium, against BoltFFI's
  0.5 and 1.3.
- **Benchmark first.** Measure every wrapper before and after, and add a Rust
  microbenchmark of the fork's lift and lower for `Vec<u8>` beside the patch it
  proves.
- **Bulk copy in the fork's core.** UniFFI lifts and lowers `Vec<u8>` through
  the generic `Vec<T>` path, one byte at a time with a bounds check each
  (`uniffi_core` `ffi_converter_impls.rs`), in both directions
  ([mozilla/uniffi-rs#2846](https://github.com/mozilla/uniffi-rs/issues/2846)).
  That path covers arguments, record fields (`MoqFrame.payload`, video and
  audio frames), and returns (`read_frame`, `recv_datagram`). Make it one
  `memcpy`. The serialized format is unchanged, so no generator or binding API
  changes. Two copies remain for records and returns (the wrapper's container
  to `RustBuffer`, then `RustBuffer` to `Vec<u8>`); removing the second is
  [zero-copy returns](/quest/m2/ffi-bytes-return.md).
  [moq-dev/moq#2946](https://github.com/moq-dev/moq/issues/2946) already
  removed the per-byte copy on the Python side.
- **The C++ generator copies per byte on the foreign side too**
  (`bytes_conv.cpp`, `RustStream >>`), capping it at 77 MB/s at 1 MiB. Fix it
  here, since it is the same problem.
- **`&[u8]` arguments only if the numbers say so.** UniFFI 0.32 makes `&[u8]`
  arguments zero-copy, but only for sync calls that take bytes directly, such
  as the `write(payload)` methods. It cannot be used in records, async calls,
  or callback interfaces. Adopting it means fixing the Go and C++/C generators
  (both lower to `RustBuffer` where the declaration expects `ForeignBytes`, so
  they would not compile) and Dart (copies and never frees). Kotlin's parameter
  type would become `ByteBuffer`. Do it only if the benchmark shows the
  remaining copy matters after the bulk copy; otherwise record the numbers and
  skip it.

Public API: none unless `&[u8]` lands, which changes Kotlin's parameter type.

## Required

- [Import UniFFI](/quest/m1/uniffi.md) - the fork the core patch lands in
- [Binding benchmark](/quest/m1/ffi-bench.md) - the before and after numbers
