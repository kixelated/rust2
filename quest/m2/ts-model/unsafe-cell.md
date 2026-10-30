# [M] std's UnsafeCell

## Goal

rust2-ts translates std's `UnsafeCell<T>`, its type and its `new`, `get`, `get_mut`, and
`into_inner`, with the pointer `get` gives read and written as raw pointers are, so
crossbeam-epoch's own `OnceLock<Collector>` static translates, proven by differential
tests against native Rust.

## Plan

Left by [#262](https://github.com/moq-dev/rust2/pull/262) on 2026-10-30: std's `Once`
translates, so crossbeam-epoch's `OnceLock` is refused only at its
`UnsafeCell<MaybeUninit<T>>`: the field at `crossbeam-epoch-0.9.21/src/sync/once_lock.rs:11`
and `get` at lines 64 and 77. The model's census refuses std's `UnsafeCell` 10 times, first
in crossbeam-epoch's own wrapper over it at `src/lib.rs:101`, and in foldhash's seed.

- A static's `UnsafeCell` already reads from memory as its contents; its type and calls
  are what remain.
- `get` gives a `*mut T`, which is what a `&mut T` is: the object, or an `rt.Place` where
  JS copies `T`, as `rt.Cell` is for `Cell<T>`.

Public API: the translator's output.
