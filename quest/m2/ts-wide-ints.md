# [M] Wide integer spike

## Goal

A report, from the [TS benchmark](/ts/README.md#bench), picks how
[rust2-ts](/ts/README.md) represents `u64`, `i64`, `u128`, and `i128`, and the
translator emits the winner for all four, exact at full width and checked on
overflow, proven by the existing differential tests.

## Plan

Decided with the maintainer on 2026-10-23: the merged design keeps `bigint`
until measured; this is that measurement. Candidates:

- `bigint` with `BigInt.asUintN`/`asIntN`, today's baseline. TurboFan lowers a
  visibly truncated 64-bit op to native word64, but plain `bigint` arithmetic
  stays heap-allocated.
- A boxed immutable hi/lo object. Simple to emit; it allocates per op (Scala.js's
  boxed `Long` measured 3-5x slower than `Int`).
- Unboxed hi/lo: the translator, which knows every type, scalar-replaces each
  value into two `uint32` numbers in locals, fields, and arguments, as Scala.js
  1.20 does. An object appears only at the API edge, typed `{ hi: number; lo:
  number }` plus a type-level `unique symbol` brand so `tsc` tells `u64` from
  `i64` at zero runtime cost.
- `u128` and `i128` use the same family with more limbs (4x32 bits, or fewer
  wider limbs if measured faster), rather than a separate path. moq-net uses
  them for `Duration` millis and cache accounting.

Measure the codec's varint and message paths and an arithmetic-heavy `lang`
case, plus output size and readability. Record the result in `ts/README.md`'s
Design section, replacing the `bigint` note.

Public API: the translator's output for 64- and 128-bit integers.

## Required

- [Faster translator runtime](/quest/m2/ts-runtime-perf.md) - removes the costs at every width first, so the spike measures only the representation
