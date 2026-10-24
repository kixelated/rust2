# [S] Diverging calls through closures and generics

## Goal

[rust2-ts](/ts/README.md) output passes `tsc` and matches native Rust in two cases
[#28](https://github.com/kixelated/rust2/pull/28) left: a call to a closure that
returns `!`, and a generic instantiated at `!`.

## Plan

- A closure returning `!` is an unannotated `const` arrow, so `tsc` does not see a
  call to it diverge: a tail `f(b)` in a function with a result fails `tsc`.
- A generic instantiated at `!` (edition 2024's fallback, as in `id(panic!())`)
  lowers its body with `Dest::Discard` and fails TS2534, and its argument is still
  `undefined`.

Public API: the translator's output for these shapes, which fail `tsc` today.
