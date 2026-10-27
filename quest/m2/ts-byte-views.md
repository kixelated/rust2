# [L] Byte slices without objects

## Goal

Splitting a `&[u8]` in translated code allocates nothing, so the codec's decode on the
[TS benchmark](/ts/README.md#bench) moves toward moq's hand-written cursor, with every
differential test still passing.

## Plan

`&[u8]` is an `rt.ByteSlice`, a range of a `Uint8Array`, so a split makes an object
rather than a view. What decode still pays per varint:

- `split_first_chunk`'s chunk is a `&[u8; N]`, which is a `Uint8Array` view of the
  slice's bytes: one per multi-byte varint. A chunk only indexed could read the slice in
  place, or a reference to a byte array could be a `ByteSlice` too.
- Each split's rest is a new `ByteSlice`, and its tuple and `Result` are objects. Byte
  slices in locals, parameters, and fields could be scalar-replaced as `(data, start,
  length)`, as `u64` halves are, materializing a `ByteSlice` only where one escapes.
  A microbenchmark of QUIC varint decode measured offsets in locals about 2x faster than a
  `ByteSlice` per split in V8 and about the same in JavaScriptCore.
- Iterating a byte slice makes one view, which `rt.iterSlice` and stepping in place read.

Measure Node, Bun, and Chrome, and Firefox once the bench has it; Chrome decides.

Public API: the translator's output for byte slices.
