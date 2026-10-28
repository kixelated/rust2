# [M] Shared ownership in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates `Arc`, `Rc`, and `Weak` with Rust's exact
counts: `strong_count`, `weak_count`, and `upgrade` match native, and the last
strong drop runs `Drop` where Rust runs it. `Mutex`, `RwLock`, atomics,
`OnceLock`, `OnceCell`, `Cell`, and `RefCell` translate as the single-threaded
values they are in JS, proven by differential tests.

## Plan

Decided with the maintainer on 2026-10-23: deterministic reference counts in
`rt.ts`, not GC-backed `WeakRef`. kio's `WaiterList` frees slots when
`Weak::upgrade` fails and reads `weak_count`, so drop timing is observable.

The async spike ([#16](https://github.com/kixelated/rust2/pull/16)) landed what its
kio-shaped fixture needs, in `rt/task.ts`: `rt.Arc` counts strong references and
drops its value with the last, `rt.Weak` upgrades while one is left, and `rt.Mutex`
is its own guard, which a lock gives as `Ok`: the translator aborts on panic, as wasm32
does, so nothing poisons a mutex. `rt.Atomic` holds a `bool` or an integer, and its operations check
their orderings as std does. `rt.OnceCell` and `rt.OnceLock` hold their `Option<T>`, and
a `OnceLock` initialized from within its own initialization panics as wasm32's does;
their `Clone`, `Debug`, `PartialEq`, `get_mut`, and `get_or_try_init` are still refused.
Left: `Rc`, `weak_count` (a dropped `Weak` counts nothing yet), `RwLock`, `Cell`,
`RefCell`, and `Mutex::try_lock` (kio's `Lock` debugs through it).

- `Send` and `Sync` bounds have no meaning in JS; accept them.
- Keep Rust's failure modes where they are defined: a `RefCell` double borrow
  panics. A re-entrant `Mutex` lock on one thread is unspecified in Rust; pick
  a loud failure (panic) and document it.
- The census of moq-net's [model](/quest/m2/ts-model/README.md) on 2026-10-27 adds:
  `thread_local!` (`LocalKey::with`) and `thread::panicking`, which is always false
  where a panic aborts.
- A `static` atomic (`static NEXT: AtomicU64`, 2 in moq-net and 1 in kio) is still
  refused as a constant of type `&Atomic`: a `static` with interior mutability needs one
  module-level value that every use shares.
- Measure the counter overhead on the [TS benchmark](/ts/README.md#bench).

Public API: the translator's output for shared and interior-mutable types.
