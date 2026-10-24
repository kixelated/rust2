# [M] Faster translator runtime

## Goal

The translated codec runs measurably faster on the
[TS benchmark](/ts/README.md#bench), with every differential test still
passing and no change to the semantics the subset documents. The report
attributes the remaining decode gap to moq's hand-written TS, workload by
workload.

## Plan

Decided with the maintainer on 2026-10-23. Candidates found in `rt.ts` and the
codec's output, each kept only if the benchmark shows it wins; no target is set
up front.

The first benchmark (PR #7) puts decode at 20-140x slower than hand-written,
with costs at every width, so this quest runs before the
[wide integer spike](/quest/m2/ts-wide-ints.md) and gives it a cheaper baseline.
Start with the costs it found:

- Every fallible call allocates a `Result`, and `?` allocates a fresh `Err`
  instead of returning the incoming one. This quest may replace the
  `{ ok, value } | { ok, error }` shape with a non-allocating one if it measures
  faster; the types at the API edge stay as they are.
- `fromBeBytes`/`toBeBytes` and `leadingZeros` loop in `bigint` even for `u16`
  and `u32`, and `from_be_bytes([a, b])` builds a `Uint8Array.of` first. Read and
  write bytes directly or through `DataView`.
- Decode takes a `subarray` view and returns a tuple per varint.

Then the rest:

- Checked ops on integers of 32 bits or less compare a number against `bigint`
  bounds, and `div`, `rem`, shifts, and the `checked`/`wrapping`/`saturating`
  families convert to `bigint`. Use number-only checks (`>>> 0`, `| 0`,
  `Math.imul`).
- Fieldless variants of data enums allocate `{ tag }` at each use; share one
  frozen object per variant.
- `rt.copy` is a generic deep clone; emit a typed copy per `Copy` type.

Leave the 64-bit representation to the wide integer spike.

Public API: the translator's output and `rt.ts`.
