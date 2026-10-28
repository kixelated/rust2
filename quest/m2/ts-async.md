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

std's future helpers translate: `Poll`'s methods, `Poll<Poll<T>>`, `pin!`, `poll_fn`, `future::pending`,
`Waker::from(Arc<W>)`, and `Waker::noop`, and so do `Box::pin` and `dyn Future`. What
moq-net still needs, by its census from MIR at the pinned revision:

- Async closures, which only moq-net's tests write today.
- kio's `Tasks`, whose `RawWaker` vtable and `Box<dyn FnMut>` tasks are
  [bindings](/ts/README.md#bindings). The rest of kio's API translates from source
  across crates; what it still refuses is std's.
- The edge: `rt.spawn`'s shape for callers, and cancelling a spawned task.
- `kio::Waiter::poll_future::<()>` does not normalize in moq-net's census, since `()` is
  no `Future`. It is reached from `util.rs`'s `future_task` closure, itself refused as a
  `Box<dyn FnMut>`, and the same call translates in the fixture
  ([#141](https://github.com/kixelated/rust2/pull/141),
  [#183](https://github.com/kixelated/rust2/pull/183)). Find where polymorphic lowering
  makes the `()`.

An async block that captures part of a variable, which THIR refused 47 times, is a
capture like any other in MIR.

Public API: the translator's output for async code.
