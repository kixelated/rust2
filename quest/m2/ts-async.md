# [M] Async in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates the async code in moq-net's model and kio, keeping
Rust's lazy futures: nothing runs until polled, and dropping a future cancels it.
Differential tests against native Rust prove the same polls, wakes, drops, and results.

## Plan

Decided by the MIR spike ([#36](https://github.com/kixelated/rust2/pull/36)): an async
body is the state machine rustc's coroutine transform makes, lowered from MIR. Each
coroutine instance is a class of its state, its captures, and the locals it keeps
across an `.await`; its `poll` is the transform's resume function, a loop over a
`switch` on the next block since it enters loops in the middle, and its
`[Symbol.dispose]` is the transform's drop, which drops what the state it is in holds.
A handoff through 1, 4, and 16 async functions measured 1.5-2.7x faster than #16's
generators in Node, 1.8-5.5x in Bun, and 1.4-3.5x in Chrome, and past depth 1 faster
than moq's hand-written promises. `rt.spawn` stays.

What moq-net still needs, by its census from MIR at the pinned revision:

- `Poll`'s helpers (about 340 calls: `is_ready`, `is_pending`, `map`, `map_ok`,
  `map_err`), `pin!` (50 with `PinMacroHelper`), `poll_fn` (31), `future::pending`
  (5), and `Box::pin` (29, a boxed future that binds to hand-written TS at the
  boundary, per the [model](/quest/m2/ts-model.md)).
- `?` inside a function that returns `Poll`, as `ready!(..)?` writes it: 284 sites.
- Async closures.
- kio's own API, reached across crates (`Pending`, `Consumer::poll`,
  `Waiter::poll_future`, `Tasks`), with the atomics, `OnceLock`, and coop budget behind
  it, which the model quest brings along.
- The edge: `rt.spawn`'s shape for callers, and cancelling a spawned task.

An async block that captures part of a variable, which THIR refused 47 times, is a
capture like any other in MIR.

Public API: the translator's output for async code.

## Required

- [Lower bodies from MIR](/quest/m2/ts-mir.md) - async bodies come from the coroutine transform
- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - async bodies hold the values that drop it still refuses
