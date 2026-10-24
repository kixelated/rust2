# [M] Faster translator runtime

## Goal

The translated codec runs measurably faster on the
[TS benchmark](/ts/README.md#bench), with every differential test still
passing and no change to the semantics the subset documents.

## Plan

Decided with the maintainer on 2026-10-23. Candidates found in `rt.ts` and the
codec's output, each kept only if the benchmark shows it wins:

- Checked ops on integers of 32 bits or less compare a number against `bigint`
  bounds, and `div`, `rem`, shifts, and the `checked`/`wrapping`/`saturating`
  families convert to `bigint`. Use number-only checks (`>>> 0`, `| 0`,
  `Math.imul`).
- `fromBeBytes`/`toBeBytes` and `leadingZeros` loop in `bigint`, and varint
  paths allocate temporary arrays per call. Read and write bytes directly or
  through `DataView`.
- `?` allocates a fresh `Err` instead of returning the incoming one.
- Fieldless variants of data enums allocate `{ tag }` at each use; share one
  frozen object per variant.
- `rt.copy` is a generic deep clone; emit a typed copy per `Copy` type.

The 64-bit representation is [its own spike](/quest/m2/ts-wide-ints.md); leave
it alone here.

Public API: the translator's output and `rt.ts`.
