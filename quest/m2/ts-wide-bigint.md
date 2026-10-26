# [M] Wide-integer runtime off bigint

## Goal

[rust2-ts](/ts/README.md)'s 64- and 128-bit runtime stops going through `bigint` on the
paths moq-net's code reaches, each change kept only if the
[TS benchmark](/ts/README.md#bench) measures it faster, with the wide differential sweep
still passing.

## Plan

The `time/u128` workloads, moq-net's `Timestamp::convert`, still take 8-12x the
hand-written `bigint` route's time after products moved onto halves. What remains there,
and elsewhere:

- `u128` and `i128` division and remainder of limbs by a number go through `bigint`,
  where a long division by a 32-bit divisor, as `u64`'s, would not.
- Comparing a number with limbs boxes the number into limbs, though each value has one
  form: limbs lie past every number, on the side their sign says.
- `checked_add`, `checked_sub`, and the saturating ones go through `bigint` for every
  value, numbers included; moq-net's `Timestamp::checked_add` is one.

Measure each against main's translation with `just ts bench --route`, and profile the
`time/` workloads first: what dominates may not be on this list.

Public API: none; the runtime's internals.
