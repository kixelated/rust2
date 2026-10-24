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

What still fails translation, each with its span (the reject fixture has one of each),
ranked by the census of moq-net at the pinned revision (kio's has 26 in all):

- Dropping a value whose drop no `[Symbol.dispose]` carries (657): a `Result`, an
  `Option` or `Poll` of one, an enum or tuple holding one, a `Vec` of one. Each needs
  glue of its own, switching on the variant.
- A method taking `self` by value that drops (155), which can't leave `this` undefined.
- A temporary that drops, unless a call or binding takes it whole (280): borrowed, a
  field or index of one, a `match`, `if let`, `let`, or let chain that moves only part
  of it, and a guarded arm that binds one. Rust drops it at the end of its statement, or
  of the `match`.
- A `for` loop over values that drop (60): each at the end of its iteration, and the
  rest on a `break`.
- Struct update syntax from a value that drops (16), a closure capturing one (13), a
  generic type that implements `Drop` (11), and a parameter that drops bound by a
  pattern.
- A value that drops, moved ahead of an `.await` in the same call (3) or in the place
  an assignment writes it to, which a cancel would leave in a temporary no scope drops,
  and an `Arc` or `Mutex` holding a borrow of a value that drops, which JS would drop
  as the value itself.

Watch the cost: each move of a value that drops writes `undefined` to where it was, and
each scope holding one becomes a `try`/`finally`. A pass that proves a variable moved on
every path, or on none, would drop most of both; measure it on the
[TS benchmark](/ts/README.md#bench) before adding it.

Public API: the translator's output for values that drop.
