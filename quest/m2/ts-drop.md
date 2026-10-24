# [M] Drop in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates every value that drops in moq-net's model and kio:
each `drop()` runs where Rust drops it, in Rust's drop order, including on early
return, `?`, panic, and a cancelled future, proven by differential tests against
native Rust.

## Plan

Decided by the async spike ([#16](https://github.com/kixelated/rust2/pull/16)) on
2026-10-24: drop points come from the THIR lowering's own scopes, not MIR. What landed
there: a class that drops gets `[Symbol.dispose]` as drop glue (its `Drop` impl, then
each field); a variable drops in a `finally` at the end of its scope; a move leaves
`undefined` behind, so a variable moved on some paths drops on the others. The task
fixture's trace test proves it on kio's waiters, wakers, and channels.

Each of these still fails translation with its span (the reject fixture has one of
each), and the census of moq-net and kio says which to lift first:

- A temporary that drops, unless a call or binding takes it whole: borrowed, a field
  or index of one, or a `match` or `if let` that moves only part of it. Rust drops it
  at the end of its statement, or of the `match`.
- A closure that captures a value that drops, which dropping the closure would drop.
- A `for` loop over values that drop: each drops at the end of its iteration, and the
  rest on a `break`.
- An enum whose variants hold values that drop, a tuple or array of them, a generic
  type that implements `Drop`, a method taking such a `self` by value, and struct
  update syntax from one.

Watch the cost: each move of a value that drops writes `undefined` to where it was, and
each scope holding one becomes a `try`/`finally`. A pass that proves a variable moved on
every path, or on none, would drop most of both; measure it on the
[TS benchmark](/ts/README.md#bench) before adding it.

Public API: the translator's output for values that drop.
