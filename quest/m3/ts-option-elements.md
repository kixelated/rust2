# [S] Vec and slice accessors of elements whose None is rt.None

## Goal

`Vec::pop`, `slice::last`, and std's other `Vec` and slice functions that return an
`Option` of an element translate where that `None` is `rt.None`, such as of a
`Vec<Option<u8>>`, keeping `None` apart from `Some(None)`, proven by a differential test
against native Rust.

## Plan

Each is refused at its span today, as `Vec::pop` is in the reject fixture's `iterators`:
its runtime function returns `undefined` for no element, which `Some(None)` is. Audit the
rest (`last_mut`, `first_mut`, `get_mut`, `split_first`, `split_last`, `pop_if`) and pass
each the `None` last, as `slice::first` and `get` already take it.

Public API: the translator's output.
