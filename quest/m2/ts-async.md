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

What moq-net and kio still need, by the census:

- Async blocks and closures. An async closure fails translation; an async block that
  captures a number by `&mut` must too, since its generator would change a copy.
- The rest of kio's machinery: atomics, `OnceLock`, the coop budget's thread-local,
  `Tasks`, and `poll_fn`, `ready`, and `pending`.
- Boxed futures, which bind to hand-written TS at the boundary
  ([model](/quest/m2/ts-model.md)).
- The edge: `rt.spawn`'s shape for callers, and cancelling a spawned task.

If a profile of moq's client finds await overhead, `yield*` between translated futures
would skip each level's `poll`; measure it on the [TS benchmark](/ts/README.md#bench).

Public API: the translator's output for async code.

## Required

- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - kio's waiters and wakers drop on paths the spike refused
