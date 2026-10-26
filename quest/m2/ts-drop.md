# [M] Drop in the TypeScript translator

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
`clear`, `unwrap_or`'s fallback) takes a dropper for each such type after its own
arguments (`drop_fn` and `droppers` in `drop.rs`). The task fixture's `drops.rs` checks
these against native Rust, natively unwinding too.

What still fails translation, by the census at the pinned moq revision (moq-net's, then
kio's):

- An iterator that owns a value that drops (124, 6), and the collection methods that
  make one (19): `drain`, `into_keys`, and `into_values`. Most are a `for` loop over a
  `Vec` or a map, and a `collect` of cloned `Path`s. An `rt.Iter` drops nothing it
  skips, consumes, or still holds when it drops. Sources that own items need droppers
  and a `[Symbol.dispose]` for what remains; adaptors drop what they discard (`filter`'s
  rejects, `skip`, `nth`, `last`, `count`, `zip`'s unmatched item, `max_by_key`'s
  losers) and what they hold (`Peekable`'s item, `Flatten`'s inner iterators); a
  consumer that takes the iterator by value drops it, also when a closure panics. Look
  out for `collect`, whose order of drops on a panic may differ between std's in-place and
  generic paths, and for closures an adaptor holds, which must drop with it or refuse.
- A closure that owns a value that drops, dropped without a call that takes it (10, 0).
- A `Drop` impl that panics: Rust drops the rest of the value and its container's other
  elements, but the runtime's droppers and a class's `[Symbol.dispose]` stop at it.

Watch the cost: a function whose cleanup does something JS can see sets a variable
before each call that can panic and wraps its body in a `try`; measure any change on
the [TS benchmark](/ts/README.md#bench).

Public API: the translator's output for values that drop.
