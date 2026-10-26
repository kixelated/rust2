# [M] Drop in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates every value that drops in moq-net's model and kio:
each `drop()` runs where Rust drops it, in Rust's drop order, including on early
return, `?`, panic, and a cancelled future, proven by differential tests against
native Rust.

## Plan

Decided by the MIR spike ([#36](https://github.com/kixelated/rust2/pull/36)): drop
points come from MIR's drop elaboration, which puts each drop where Rust runs it, with
a flag where a value moved on some paths only, so the lowering adds nothing of its own:
no `finally` per scope, and no `undefined` left by a move. A panic unwinds through the
cleanup blocks of `-C panic=unwind` MIR, which a function runs from a `catch` (the
task fixture's unwinding test checks them against native unwinding). An enum, tuple,
or `Result` that drops is dropped by glue lowered from rustc's own `drop_in_place`
shim.

What still fails translation from MIR, by the census of moq-net at the pinned revision
(kio's has 9 in all):

- An iterator that owns a value that drops (95): an `rt.Iter` drops nothing it skips,
  consumes, or still holds when it is dropped.
- Dropping a value whose glue is std's (128): a `Vec` of a value without
  `[Symbol.dispose]`, such as an enum, and the instances of generic enums a census
  lowers over their type parameters.
- A std mapping that may discard an operand that drops (20): `keeps_operands` in
  `library.rs` refuses any operand that drops, even one a mapping always keeps;
  it should name the operands each mapping may discard instead.
- A generic type that implements `Drop` (11).
- A collection holding a value that drops (279): dropping it (118), and the methods
  that may drop what it holds or is given (161), which are `clear`, `retain`,
  `truncate`, `drain`, a `remove` of a key that drops, which drops the key the map
  held, and an `insert` or `entry` given a key that drops, such as a `Path`, which
  drops that key if the map holds an equal one.

THIR's refusals that MIR's elaboration closes, for comparison: 657 drops of a value
without glue, 280 temporaries that drop, 60 `for` loops, 112 moves out of other places,
and 75 more in patterns, closures, and struct updates.

Watch the cost: a function whose cleanup does something JS can see sets a variable
before each call that can panic and wraps its body in a `try`; measure any change on
the [TS benchmark](/ts/README.md#bench).

Public API: the translator's output for values that drop.
