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
the translator's ([design](/ts/README.md#design)): correctness first, speed second,
and bundle size third, with readable output not a goal. So the state machines'
1.4-2.6x speed weighs ahead of their roughly 18% larger bundle; measure the size
as the consumer's final minified JS.

Why, from #16:

- Its MIR state machines were 1.4-2.6x faster per handoff than the generators it
  landed, and about 18% larger gzipped.
- THIR scope drops drew 10 review rounds of small drop-coverage bugs. MIR's drop
  elaboration is exact by construction.

What #16 found against MIR, to answer rather than assume: its prototype
re-implemented places, rvalues, constants, closures, and `format_args!` at the MIR
level, and under wasm32's `panic=abort` MIR has no cleanup paths, so a JS
exception would skip drops that THIR's `finally` blocks run. That its output read as
basic blocks no longer counts against it.

Public API: none unless the translator moves; then its whole output.
