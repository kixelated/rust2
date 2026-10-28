# [S] Checks on the differential harnesses

## Goal

The differential harnesses can't silently test less than they claim: a test fails when the
task fixture's step list in Rust (`harness.rs`) and TypeScript (`tests/task.ts`) drift
apart, when a scripted case's literal op bytes stop picking the step or case it names, and
an audit of `lang`'s `refs` case shows which of the place lowering's branches it reaches.

## Plan

Planned on 2026-10-28. Both lists are matched by order today, so a step added to one side
only shifts every op after it, and a script written as bytes rather than by name keeps
running whatever its bytes now pick.

- Compare the lists by name in a test, rather than generating one from the other.
- The `refs` audit adds cases for the branches it misses, such as #148's review asked:
  reborrows through a reassigned reference, and indexed borrows.

Public API: none.
