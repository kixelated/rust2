# [M] Test selection from MIR call graphs

## Goal

A report of how many tests a function-level selector would skip on moq's history,
beyond the package-level selection moq's CI already does: for each real change, the
tests whose MIR call graph cannot reach a changed function, and how much CI CPU
time skipping them would save.

## Plan

Decided with the maintainer on 2026-10-25: plan it as research. The baseline is
moq's existing crate-level selection: `just ci` and its merge queue build and test
only the packages a branch changed plus their dependents (moq's `sh/dispatch.sh`
and `sh/changed.sh`).

- Build each test's reachable set from monomorphized MIR with
  [rust2-driver](/driver/src/lib.rs). Trait objects and function pointers make the graph
  inexact, so assume any reachable impl of the trait or signature can be called.
- A test that reads files, the environment, or the clock depends on more than
  code; count how often that would force a test to run anyway.
- The report recommends whether to build a selector, and estimates its cost.

Public API: none.
