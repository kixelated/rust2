# [S] Drop in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates every value that drops in moq-net's model and kio:
each `drop()` runs where Rust drops it, in Rust's drop order, including on early
return, `?`, panic, and a cancelled future, proven by differential tests against
native Rust.

## Plan

Decided by the MIR spike ([#36](https://github.com/kixelated/rust2/pull/36)): drop
points come from MIR's drop elaboration, which puts each drop where Rust runs it, with
a flag where a value moved on some paths only. A panic unwinds through the cleanup
blocks of `-C panic=unwind` MIR, which a function runs from a `catch`. A value without
`[Symbol.dispose]`, such as an enum or a tuple, drops through glue lowered from rustc's
`drop_in_place` shim. A runtime function that may drop what std's would (a collection's
`clear`, `unwrap_or`'s fallback, an adaptor's rejects) takes a dropper for each such
type after its own arguments (`drop_fn` and `droppers` in `drop.rs`), and a consumer
that takes an iterator by value takes `rt.drop` for it. A closure that owns a value that
drops carries `[Symbol.dispose]` (`rt.closure`), and one that `Fn` or `FnMut` call drops
after a call through `FnOnce` (`rt.callOnce`, and `rt.once` for std's). The task
fixture's `drops.rs`, `drops/iters.rs`, and `drops/closures.rs` check these against
native Rust, natively unwinding too.

Nothing that drops fails translation in the census at the pinned moq revision. Refused
with none in the census, each where std's order is one JS's can't follow yet:
a closure that owns a value that drops, given to std's iterators or a collection's
methods, which hold or call it (an adaptor drops it after its iterator, as std's fields
do, and each consumer where its `fold` path does);
`into_keys` and `into_values` of a hash map whose other half drops (`fold` drops it
after the half it yields, `next` before); `zip` of two `TrustedRandomAccess` iterators
of items that drop; a `flatten` whose inner iterator holds items once ended, or that a
`fold` reads where both its source and inner iterators drop; a B-tree's `collect` of
entries that drop, sorted by the crate's own `Ord`; a `collect` in place of a `zip`
whose second iterator drops; `successors` of items that drop, which holds the next one;
and `DebugList::entries` of items that drop.

Watch the cost: a function whose cleanup does something JS can see sets a variable
before each call that can panic and wraps its body in a `try`; measure any change on
the [TS benchmark](/ts/README.md#bench).

Public API: the translator's output for values that drop.
