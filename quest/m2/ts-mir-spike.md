# [L] MIR lowering spike

## Goal

A report decides whether [rust2-ts](/ts/README.md) lowers from MIR instead of THIR.
A port of the whole lowering to MIR translates the codec and task fixtures (plain
code, drops, and async state machines), and is measured against the THIR lowering
on bundle size, `just ts bench` speed, exact drop traces (the task fixture's trace
diff), and how many census refusals move. If MIR wins, the translator moves to it;
otherwise the port is deleted.

## Plan

Decided with the maintainer on 2026-10-24: merge the async spike
([#16](https://github.com/kixelated/rust2/pull/16)) on THIR, then run this spike
over the whole lowering, on those two fixtures. The priorities that decide it are
correctness first, bundle size second, and readability last.

Why, from #16:

- Its MIR state machines were 1.4-2.6x faster per handoff than the generators it
  landed, and about 18% larger gzipped.
- THIR scope drops drew 10 review rounds of small drop-coverage bugs. MIR's drop
  elaboration is exact by construction.

What #16 found against MIR, to answer rather than assume: its prototype
re-implemented places, rvalues, constants, closures, and `format_args!` at the MIR
level; the resume dispatch jumps into the middle of loops, so the output read as
basic blocks; and under wasm32's `panic=abort` MIR has no cleanup paths, so a JS
exception would skip drops that THIR's `finally` blocks run.

Public API: none unless the translator moves; then its whole output.

## Required

- [Async and drop spike](/quest/m2/ts-async-spike.md) - the task fixture, its trace test, and the THIR lowering this compares against
