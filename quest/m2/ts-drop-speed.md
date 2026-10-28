# [M] Values that drop, at the speed of values that don't

## Goal

[rust2-ts](/ts/README.md) output for a type that drops runs as fast as it did before the
type dropped. The [TS benchmark](/ts/README.md#bench)'s async handoff, at depth 1 and 16, is
within noise of its time before `Weak` dropped
([#198](https://github.com/kixelated/rust2/pull/198)) on Node, Bun, Chrome, and Firefox,
measured against an A/A route. Every value still drops where Rust drops it, proven by
the existing differential tests.

## Plan

#198 made kio's `WaiterList` drop its `Weak`s, and the async handoff took 20-40% longer at
depth 1. The counts cost nothing measurable; the drop codegen every type that drops gets
does:

- `rt.drop(x)` is one megamorphic site that looks up `[Symbol.dispose]` and calls it
  through `.call`. A value whose type always has the method calls it directly.
- A class takes its fields as one object, `new T({ a, b })`, which allocates twice.
  Take them positionally.
- A loop over an owned `Vec` steps an index so that what an early exit leaves can drop
  after it. A loop with no early exit is `for…of`, and drops nothing after it.
- Drop flags: rustc's drop elaboration sets one only where a value is moved on some paths
  and not others. Measure whether any is worth removing where a path-sensitive drop
  could replace it.

Public API: a class's constructor and the runtime's `drop`.
