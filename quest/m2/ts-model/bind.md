# [M] Bind to hand-written TypeScript

## Goal

[rust2-ts](/ts/README.md) calls hand-written TypeScript that `ts.toml` names in place of a
Rust function or type, typed by its Rust signature and checked by `tsc`, so a boundary the
subset refuses for good translates around it.

## Plan

Decided with the maintainer on 2026-10-23 and 2026-10-27: `unsafe` and boundary trait
objects bind to hand-written TS through config, and a dependency containing `unsafe` is
bound by the consumer. The mechanism is general; nothing in it names moq.

What the [model](/quest/m2/ts-model/README.md) needs bound, from its census on 2026-10-27:

- `Box<dyn Future>` in moq-net's `util.rs` and kio's `Tasks`, and kio's `RawWaker` vtable.
- moq-clock's browser clock and `SystemTime`.
- `rand::rng`.
- `flate2`'s inflater, bound to a synchronous JS inflater such as fflate, since browsers
  have no synchronous DEFLATE.
- `bytes::Bytes` and `BytesMut` (86 refusals), which contain `unsafe`.

moq-net's and kio's internal `unsafe` gets safe `cfg` twins in moq instead.

- A binding's TS is type-checked against the Rust signature's translated type, and a
  missing or mismatched binding fails the translation with the Rust item's span.
- Report the model's census before and after.

Public API: `ts.toml` gains bindings.
