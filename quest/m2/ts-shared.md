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
kio-shaped fixture needs, in `rt/task.ts`: `rt.Arc`, an `Arc` or an `Rc`, counts
strong and weak references and drops its value with the last strong one, `rt.Weak`
upgrades while one is left, and `rt.Mutex` is its own guard, which a lock gives as
`Ok`: the translator aborts on panic, as wasm32 does, so nothing poisons a mutex. A
guard's drop unlocks it, `try_lock` while held gives `WouldBlock`, and a re-entrant
`lock` panics with wasm32's message. `new_cyclic` gives its function a `Weak` that
upgrades once the function returns. `rt.Atomic` holds a `bool` or an integer, and its
operations check their orderings as std does. `rt.OnceCell` and `rt.OnceLock` hold
their `Option<T>`, and a `OnceLock` initialized from within its own initialization panics as wasm32's does;
their `Clone`, `Debug`, `PartialEq`, `get_mut`, and `get_or_try_init` are still refused.
`rt.Cell` is the place that holds its value. `rt.RefCell`, a `RefCell` or an `RwLock`,
is its own guard and counts its borrows: a conflicting borrow panics, and a conflicting
lock aborts, each with wasm32's message. Their `get_mut`, `into_inner`, `try_borrow`,
`Ref::map`, and `Ref::clone` are still refused.
A `thread_local!` key is an `rt.LocalKey`, a module's value that makes its value on
first use, as wasm32's `LazyStorage` does, and never drops it. A `RefCell` key's `set`
and `with_borrow_mut` are still refused: std's bodies write through a `&mut`, which a
body of std's can't yet.
Left: `static` atomics, below, which a module's value, as a thread-local's, can hold.

- `Send` and `Sync` bounds have no meaning in JS; accept them.
- A `static` atomic (`static NEXT: AtomicU64`, 2 in moq-net and 1 in kio) is still
  refused as a constant of type `&Atomic`: a `static` with interior mutability needs one
  module-level value that every use shares.
- Measure the counter overhead on the [TS benchmark](/ts/README.md#bench).

Public API: the translator's output for shared and interior-mutable types.
