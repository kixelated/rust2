# [M] Kotlin bindings: zero-copy payloads and the read path

## Goal

The generated Kotlin meets the rest of the [epic's](/quest/m1/ffi-export/README.md) rules for
every generator: it keeps the C ABI's zero-copy payloads, and its reads cost what JNI must.

## Plan

`kotlin/<prefix>.kt` and its JNI glue land with [rust2#184](https://github.com/kixelated/rust2/pull/184).
Bytes cross JNI as a `ByteArray`, one bulk copy each way, where C++ lends a `view` and owns
Rust's `buf`. A `java.nio.ByteBuffer` from `NewDirectByteBuffer` would lend Rust's buffer
without a copy, but needs an owner to free it: decide with the maintainer between a
`ByteBuffer` an `AutoCloseable` frees, a `ByteArray` kept for small payloads, or both by size.
Measure each on `just ffi bench` against the copy.

Measured on `just ffi bench`, a read of a ready 100 B frame costs about 890 ns through the
Kotlin against 410 ns through C: six JNI crossings (make, waker node, poll, free, node free,
frame free) and a global ref per read. A first poll fused into the call that makes the future,
with the waker node made only when it pends, would cut that to two; measure before landing it.

An object left unclosed leaks its Rust value. Decide whether a `java.lang.ref.Cleaner` frees
it once collected: it needs JVM 9 and Android API 33, and costs an allocation per object.

Public API: the Kotlin's bytes type, if the payload decision changes it.
