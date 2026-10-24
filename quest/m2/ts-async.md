# [M] Async in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates the async code in moq-net's model and kio, keeping
Rust's lazy futures: nothing runs until polled, and dropping a future cancels it.
Differential tests against native Rust prove the same polls, wakes, drops, and results.

## Plan

Decided by the async spike ([#16](https://github.com/kixelated/rust2/pull/16)) on
2026-10-24: an async body is a generator lowered from THIR, not a state machine lowered
from MIR. MIR's were about twice as fast per await level, but would need a second
lowering of everything an async body can hold, and read as basic blocks. What landed
there: an `async fn` returns `rt.future(body, drop, ...upvars)`, whose body is a
module-level `function*` of what it captured (a generator function made per call costs
V8 about 25 times as much), `.await` is a poll loop around `yield`, and `rt.spawn` runs
a future from a microtask as a promise. The task fixture's trace test covers a
kio-shaped channel at await depths 1, 4, and 16.

What moq-net still needs, by its census at the pinned revision:

- `Poll`'s helpers (about 270 calls: `is_ready`, `is_pending`, `map`, `map_ok`,
  `map_err`), `pin!` (47), `poll_fn` (51 with its `poll`), and `Box::pin` (17, a boxed
  future that binds to hand-written TS at the boundary, per the
  [model](/quest/m2/ts-model.md)).
- An async block that captures part of a variable (47), or borrows a number mutably
  (4); async closures.
- kio's own API, reached across crates (`Pending`, `Consumer::poll`,
  `Waiter::poll_future`, `Tasks`), with the atomics, `OnceLock`, and coop budget behind
  it, which the model quest brings along.
- The edge: `rt.spawn`'s shape for callers, and cancelling a spawned task.

If a profile of moq's client finds await overhead, `yield*` between translated futures
would skip each level's `poll`; measure it on the [TS benchmark](/ts/README.md#bench).

Decided with the maintainer on 2026-10-24: the async spike
([#16](https://github.com/kixelated/rust2/pull/16)) merges with generators from
THIR, and the [MIR lowering spike](/quest/m2/ts-mir-spike.md) then decides
whether the whole lowering moves to MIR; rewrite this plan to its report too.
Also in scope, found since:

- `?` inside a function that returns `Poll`, as `ready!(..)?` writes it: about
  224 sites in moq-net.
- A `()` future awaited as a statement fails `tsc`: `Future<void>` against
  `Future<undefined>`.

Public API: the translator's output for async code.

## Required

- [MIR lowering spike](/quest/m2/ts-mir-spike.md) - rewrite to the spike's report before starting
- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - async bodies hold the values that drop it still refuses
