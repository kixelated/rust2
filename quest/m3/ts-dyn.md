# [L] Trait objects

## Goal

[rust2-ts](/ts/README.md) translates trait objects (`&dyn Trait` and `Box<dyn Trait>`), with
Rust's dispatch and drop order, proven by differential tests against native Rust, so a
consumer neither rewrites nor binds them.

## Plan

Decided with the maintainer on 2026-10-27: a stock crate translates with no annotations or
config, so trait objects move from never to not yet. Until this lands, moq rewrites
`dyn Expiry` as an enum and binds its boxed closures ([bindings](/ts/README.md#bindings)).

- `&dyn Debug`, `&dyn Display`, and `dyn Future` already translate; build on how they do.

Public API: the translator's output.
