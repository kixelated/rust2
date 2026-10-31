# [S] Pointers that write nothing whole

## Goal

[rust2-ts](/ts/README.md) counts no whole write of a `T` where a `*mut T` is only held
as a value: given to a std atomic, or kept in memory and read back only by the crate's
own code. The pointers into a `futures::task::AtomicWaker` that moq-net's census refuses
through wasmtimer's timers, and the one into kio's `Segment` slots, translate.

## Plan

Found while making a whole write replace every part it holds, at any depth, on
2026-10-31. `Writes` in `ts/src/lower/copy.rs` counts a whole write of `T` for each
`*mut T` given to a std function `pointer::keeps` doesn't list, and for each one kept
anywhere but in a local. Both over-approximate:

- `AtomicPtr`'s methods hold the pointer as a value and never write through it. Listing
  std's `Atomic` in `pointer::keeps` drops the census refusal at kio's `log.rs:121`,
  whose `Segment` only `AtomicPtr` calls count as written whole.
- wasmtimer's `ArcList::pop` keeps a `*mut Node<ScheduledTimer>` in its `AtomicPtr`, so
  `Node` counts as written whole and, through `ScheduledTimer`, so does its
  `AtomicWaker`. That makes the four refusals at `futures-core`'s `atomic_waker.rs`
  (lines 277, 279, 311, 385). Nothing writes a `Node` whole. If a kept pointer reaches
  only crate code, whose writes `Writes` sees, and std calls, which the call site
  judges, the rule can go; find what else reaches one first.

Prove each with a reject case that a real whole write still refuses.
