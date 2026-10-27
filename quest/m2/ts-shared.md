# [M] Shared ownership in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates `Arc`, `Rc`, and `Weak` with Rust's exact
counts: `strong_count`, `weak_count`, and `upgrade` match native, and the last
strong drop runs `Drop` where Rust runs it. `Mutex`, `RwLock`, atomics,
`OnceLock`, `Cell`, and `RefCell` translate as the single-threaded values they
are in JS, proven by differential tests.

## Plan

Decided with the maintainer on 2026-10-23: deterministic reference counts in
`rt.ts`, not GC-backed `WeakRef`. kio's `WaiterList` frees slots when
`Weak::upgrade` fails and reads `weak_count`, so drop timing is observable.

The async spike ([#16](https://github.com/kixelated/rust2/pull/16)) landed what its
kio-shaped fixture needs, in `rt/task.ts`: `rt.Arc` counts strong references and
drops its value with the last, `rt.Weak` upgrades while one is left, and `rt.Mutex`
is its own guard. Left: `Rc`, `weak_count` (a dropped `Weak` counts nothing yet),
`RwLock`, atomics, `OnceLock`, `Cell`, and `RefCell`.

- `Send` and `Sync` bounds have no meaning in JS; accept them.
- Keep Rust's failure modes where they are defined: a `RefCell` double borrow
  panics. A re-entrant `Mutex` lock on one thread is unspecified in Rust; pick
  a loud failure (panic) and document it.
- Measure the counter overhead on the [TS benchmark](/ts/README.md#bench).

Public API: the translator's output for shared and interior-mutable types.

## Required

- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - the last strong drop runs `Drop`
