# [L] Byte slices without objects

## Goal

Splitting a `&[u8]` in translated code allocates nothing, so the codec's decode on the
[TS benchmark](/ts/README.md#bench) moves toward moq's hand-written cursor, with every
differential test still passing.

## Plan

`&[u8]` and `&[u8; N]` are an `rt.ByteSlice`, a range of a `Uint8Array`, so a split makes
an object rather than a view, and a chunk's bytes are read in place. What decode still
pays per varint:

- Each split's rest is a new `ByteSlice`, and its tuple and `Result` are objects. Byte
  slices in locals, parameters, and fields could be scalar-replaced as `(data, start,
  length)`, as `u64` halves are, materializing a `ByteSlice` only where one escapes.
  A microbenchmark of QUIC varint decode measured offsets in locals about 2x faster than a
  `ByteSlice` per split in V8 and about the same in JavaScriptCore.
- Iterating a byte slice makes one view, which `rt.iterSlice` and stepping in place read.

Measure Chrome, Firefox, Node, and Bun; Chrome decides.

Public API: the translator's output for byte slices.
