# [S] Closures that return an Option whose None is rt.None

## Goal

`find_map`, `filter_map`, `map_while`, `iter::from_fn`, and `successors` translate where
their closure returns an `Option` whose `None` is `rt.None`, such as `Option<Option<u8>>`,
keeping `None` apart from `Some(None)`, proven by a differential test against native Rust.

## Plan

Each is refused at its span today, as `find_map` is in the reject fixture's `iterators`: the runtime
tests the closure's result against `undefined`, which `Some(None)` is. Each needs the
`None` passed in, as the iterator consumers that return an item take it.

Public API: the translator's output.
