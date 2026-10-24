# [L] Async in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates `async fn`, `.await`, and the `Future` and `Waker`
machinery kio is built on, keeping Rust's lazy futures: nothing runs until polled.
Differential tests against native Rust prove the same poll order and results.

## Plan

Decided with the maintainer on 2026-10-23: async must keep Rust's lazy futures, so
an eager `async`/`await` mapping, as rust-js makes, is out. A future is a state
machine its executor polls; JS promises can sit at the edge, for callers.

The [spike](/quest/m2/ts-async-spike.md) picks between generators and MIR state
machines; rewrite this plan to its report before starting.

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

- [Async and drop spike](/quest/m2/ts-async-spike.md) - the lowering this builds on
- [MIR lowering spike](/quest/m2/ts-mir-spike.md) - rewrite to the spike's report before starting
- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - a dropped future drops its state
