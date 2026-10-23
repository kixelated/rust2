# [L] Async in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates `async fn`, `.await`, and the `Future` and `Waker`
machinery kio is built on, keeping Rust's lazy futures: nothing runs until polled.
Differential tests against native Rust prove the same poll order and results.

## Plan

Decided with the maintainer on 2026-10-23: async must keep Rust's lazy futures, so
an eager `async`/`await` mapping, as rust-js makes, is out. A future is a state
machine its executor polls; JS promises can sit at the edge, for callers.

Public API: the translator's output for async code.
