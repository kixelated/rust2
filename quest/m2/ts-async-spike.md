# [L] Async and drop lowering spike

## Goal

A report picks how [rust2-ts](/ts/README.md) translates async code and where it
reads drop points, from two prototypes run on the same kio-shaped channel:
JS generators lowered from THIR, and state machines lowered from MIR after the
coroutine transform. The report also decides whether the whole lowering moves
from THIR to MIR, and rewrites [Drop](/quest/m2/ts-drop.md) and
[Async](/quest/m2/ts-async.md) to the winner.

## Plan

Decided with the maintainer on 2026-10-23: prototype both, and measure one
spike for async and drop together, since both hinge on THIR versus MIR.

Research (2026-10-23):

- Nearly all of moq's async is `poll_*` functions behind one-line
  `kio::wait(|w| self.poll_x(w)).await` wrappers. The hard parts are kio's
  internals: `Waiter` holds `Arc<Waker>`, `WaiterList` holds `Weak<Waker>` and
  frees slots when `Weak::upgrade` fails, plus `Mutex`, atomics, and a
  thread-local coop budget. No `select!`, `join!`, or tokio in kio or the model.
- Generators: rustc desugars `.await` before THIR into a poll loop around
  `yield`, which maps to `cx = yield` and `gen.next(cx)`. Laziness is free, JS
  objects do not move so `Pin` is moot, and `gen.return()` runs `finally` and
  `using` disposals. Gaps: the first `next()` drops its argument, an unstarted
  generator's `return()` runs no `finally` (its captured arguments must still
  drop), and polling after completion must panic. Output stays readable.
- MIR state machines: rustc's own transform, with its drop shim giving exact
  drops on cancel, and the fastest shape. The output needs a structurizer and
  reads poorly, and lowering MIR means mapping `Pin::new_unchecked` and raw
  pointers.
- Eager `async`/`await` (rust-js's approach) is out: it breaks laziness, poll
  order, and cancel-by-drop.
- A microbenchmark of a two-level await handoff measured a hand-written state
  machine at 75 ns, generators at about 180 ns, and `async`/`await` at about
  95 ns in Node 24. Generators still sustain millions of wakeups per second.

Prototype shape:

- Translate a small kio-like `Producer`/`Consumer`, `Waiter`/`WaiterList`,
  `kio::wait`, and one `async fn` consumer loop. Each prototype gets only the
  `Arc`/`Weak` counting, `Drop`, and `Mutex` it needs.
- A deterministic FIFO executor in both Rust and TS logs every poll, wake, and
  drop; the differential test requires identical traces. Cases: a never-polled
  future has no effects, a future dropped mid-await drops in Rust's order,
  dropping an unstarted future drops its arguments, and polling after `Ready`
  panics.
- Executor integration: woken tasks run in a batch from one `queueMicrotask`;
  the coop budget yields to a macrotask; promises appear only at the edge.
- Measure on the [TS benchmark](/ts/README.md#bench) against hand-written TS
  and moq's WASM: handoff cost at await depths 1, 4, and 16, plus output size.

Land the harness and the winning prototype's tests; record the loser's numbers
in the report and delete its code.

Public API: none until Drop and Async land.
