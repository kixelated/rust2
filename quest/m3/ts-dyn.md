# [L] Trait objects

## Goal

[rust2-ts](/ts/README.md) translates trait objects (`&dyn Trait`, `Box<dyn Trait>`, and
`Box<dyn Future>`, `Pin`ned or not), with Rust's dispatch and drop order, proven by
differential tests against native Rust, so a consumer neither rewrites nor binds them.

## Plan

Decided with the maintainer on 2026-10-27: a stock crate translates with no annotations or
config, so trait objects move from never to not yet. Until this lands, moq rewrites
`dyn Expiry` as an enum and binds its boxed futures ([bindings](/ts/README.md#bindings)).

- `&dyn Debug` and `&dyn Display` already translate; build on how they do.
- A boxed future must keep Rust's laziness and cancellation as
  [async](/quest/m2/ts-async.md) defines them.

Public API: the translator's output.
