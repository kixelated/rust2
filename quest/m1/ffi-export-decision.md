# [XS] Waiting: decide whether the exporter replaces the UniFFI fork

## Goal

The maintainer decides, from [the exporter spike's report](https://github.com/kixelated/rust2/pull/41),
whether [rust2-ffi](/ffi/README.md) replaces [the UniFFI fork](/uniffi/README.md) for moq,
and so whether the fork's paused feature quests fold into it. Check: ask the maintainer.
Delete this quest once decided, rescoping or deleting each quest that requires it.

## Plan

The spike's recommendation: adopt the exporter, and fold every paused quest into it.

- It exports 87% of moq-net's types and 81% of their methods and fields with no
  attributes, and its C test publishes and subscribes to a track against the real
  library, with every read a polled future. Four more rules (tuples as out parameters,
  `Path<'static>`, a clone for a borrowed `Clone` return, and `Range`) cover most of the
  rest; the `poll_*` methods with an async twin are 39 of the 112 refused methods.
- Its frame writes cost what moq's hand-written `rs/moq-c` does (within 5%, the rest a
  timestamp's handle), so call overhead is no reason to keep a wrapper.
- What moq-ffi adds over moq-net is mostly other crates (sessions over moq-native, media
  over hang and the codecs) and a runtime to drive sessions, not reshaping moq-net.

Each paused quest, if adopted:

- [C-first bindings spike](/quest/m2/uniffi-c-first-spike.md): done in kind. The export is
  C with nothing underneath, callbacks are C, and the name clash is refused at both spans
  with a `rename` to fix it. C++ per standard layers on its C ABI, as a generator.
- [Namespaced contract](/quest/m2/uniffi-contract.md) and [checksums](/quest/m2/ffi-checksums.md):
  the export has neither. A header and library from different builds still mix silently,
  so the exporter needs one contract symbol per build, checked at load.
- [Payload bytes](/quest/m1/ffi-bytes.md) and [zero-copy returns](/quest/m2/ffi-bytes-return.md):
  done at the C ABI. A sync call borrows a payload argument with no copy, and a returned
  `Bytes` is handed over without one; each language's generator must keep that.
- [Cheaper async](/quest/m2/ffi-async.md) and [foreign poll](/quest/m1/ffi-poll-protocol.md):
  a future ready on its first poll costs three crossings (call, poll, free), not UniFFI's
  five, and the poll that finishes writes the result, so a cancel never loses a frame. A
  repeated waker is reused, not re-registered. Left: a future handle per read.
- [Code size](/quest/m2/ffi-code-size.md): moq-net's header is 1003 functions, 99 KB
  without its docs; measure per language once generators exist.
- [Kotlin calls](/quest/m1/kt-ffi-calls.md): JNI or Panama becomes the Kotlin generator's
  choice over the C ABI, rather than a patch to the fork's JNA bindings.

## Related

- [Exporter](/quest/m1/ffi-export.md) - the work that follows, if adopted
