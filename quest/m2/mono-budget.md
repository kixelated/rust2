# [M] Monomorphization budget lint

## Goal

A rust2-driver lint fails a build when a generic function's instantiations cost
more than a budget: copies times MIR size, reported with the generic's span and
its heaviest instantiating callers. moq runs it on moq-net, where it flags
`moq_clock::sim::run` before moq's
[thin executors](https://github.com/moq-dev/moq/blob/main/quest/m0/ci/thin-executors.md)
and passes after.

## Plan

Decided with the maintainer on 2026-10-25: moq adopts "thin generics" rules by
hand first, and the driver enforces them later, so the lint's rule comes from
those quests' measurements.

- **Measure, don't guess:**
  - count mono items per generic from the collector, as `-Zprint-mono-items`
    does;
  - weigh each by its optimized MIR statement count;
  - check that ranking against `cargo llvm-lines` on moq-net.
- **Budget:**
  - one crate-wide default, overridable per item with an attribute that carries a
    reason;
  - a generic whose body only forwards to a non-generic inner function stays
    under any budget.
- **Fail loud:** an error with the span, never a warning, matching the repo's
  rule.
- **Cost:** collecting mono items is work rustc already does for codegen. Report
  the lint's own time on moq-net.

Public API: a new lint in rust2-driver.
