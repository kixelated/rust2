# [L] Cheaper byte slices

## Goal

Splitting a `&[u8]` in translated code stops costing a `Uint8Array` view per
split, so the codec's decode on the [TS benchmark](/ts/README.md#bench) moves
toward moq's hand-written cursor, with every differential test still passing.

## Plan

The report in PR #15 puts views at most of what decode still costs:
about 25 ns each in V8, and a varint takes one to four (`split_first`'s rest,
`split_first_chunk`'s chunk and rest, a `lo @ ..` pattern). moq's hand-written
`Cursor` reads through an offset and makes none.

Options to measure, not a decision:

- Lower `&[u8]` to an offset and length into a shared array, as `(buf, start,
  end)` locals the translator scalar-replaces, materializing a `Uint8Array`
  only at the API edge.
- Keep `Uint8Array` but skip views nothing reads: a split whose chunk is only
  destructured into bytes, or whose rest is unused on the path taken.

Engines differ: JavaScriptCore makes views slower than V8 but small copies
faster, and V8 gives a lone small array its own buffer the first time it is
viewed (`rt.bytes` exists for that). Measure Node, Bun, and Chrome.

Public API: the translator's output for byte slices.

## Related

- [Wide integer spike](/quest/m2/ts-wide-ints.md) - the other half of decode's remaining gap
