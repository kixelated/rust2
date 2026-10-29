# [M] Kotlin reads at JNI's cost

## Goal

A ready read through the generated Kotlin costs what its JNI crossings must, not what its
allocations add: `just ffi bench`'s "frame written and read" and "payload read" lines close
most of their gap to C, each cut measured. Today, after
[#221](https://github.com/kixelated/rust2/pull/221), Kotlin takes 644-675 ns per frame
written and read against C's 401-426, and 151-169 ns per 100 B payload read against C's
14-15.

The semantics stay as they are: `close()` is idempotent and frees exactly once, the GC frees
what nobody closed, a cancelled read loses nothing, and the public API is unchanged unless the
maintainer decides otherwise.

## Plan

Found by #221, which cut the crossings per ready read from six to two. A JNI call costs only
about 5 ns over C's (a handle call: 15 ns against 7), so the rest is likely allocation and
synchronization per read. These are leads, not a profile:

- Each object's `Handle` allocates an `AtomicLong` and an `AtomicBoolean` beside itself. One
  state word, through an `AtomicLongFieldUpdater`, which every JVM and Android level has,
  would drop both.
- A ready result is written into a Rust `Vec`, copied into a new `byte[]`, and read back
  through a `ByteBuffer.wrap`, even when it is one handle or a bool.
- The GC backstop links each object into one list under a lock, at track and untrack: about
  20-30 ns per object, measured in #221. The `PhantomReference` itself costs nothing
  measurable. #221 kept the plain list: a thread-local list loses references when its thread
  exits, and a lock-free one needs the cleaner thread to sweep it.
- A payload read crosses twice (the getter, then `NewDirectByteBuffer`), makes a read-only
  view and a cleanup reference, and crosses again to free.
- Each suspend call allocates a `Signal`.

Profile allocations first, with JFR or async-profiler, rather than trusting this list. The
bench machine is noisy: compare builds in interleaved runs, as #221 did.

## Related

- [Kotlin bindings on Android](/quest/m1/ffi-export/kotlin-android.md) - whatever replaces the atomics or the list must hold on ART too
