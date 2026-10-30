# [M] Kotlin payload reads at JNI's cost

## Goal

A payload read through the generated Kotlin costs what its JNI crossings must:
`just ffi bench`'s "payload read" line closes most of its gap to C, each cut measured. After
[#231](https://github.com/moq-dev/rust2/pull/231), Kotlin takes 147-176 ns per 100 B
payload read against C's 15-16.

The semantics stay as they are: `close()` is idempotent and frees exactly once, and the GC
frees what nobody closed. The maintainer decides whether `Bytes` may copy.

## Plan

An async-profiler profile of payload reads alone, in #231, puts about 60% of the time in
`NewDirectByteBuffer`: HotSpot builds the buffer through an upcall into its constructor,
and no public API wraps a native address more cheaply before JDK 22's FFM, which Android
lacks. The `Free` registration, `asReadOnlyBuffer`, and the free crossing take most of the
rest.

A prototype that copied payloads up to 1 KiB into a heap `ByteArray`, freeing the Rust
bytes in the same crossing and registering nothing with the cleaner, measured 68 ns. Its
remaining cost was `NewByteArray` and `SetByteArrayRegion`. Open question for the
maintainer: copy below a threshold and lend above it, keep lending always, or always copy.
Folding the copy into the getter's own crossing would cut one more.

A ready result still crosses as a new `byte[]` from `NewByteArray`. A thread-local direct
buffer that Rust writes and Kotlin reads would drop it, at the cost of a reentrancy rule:
nothing may cross on that thread until the result is read.

## Related

- [Kotlin bindings on Android](/quest/m1/ffi-export/kotlin-android.md) - whatever replaces the buffers must hold on ART too
