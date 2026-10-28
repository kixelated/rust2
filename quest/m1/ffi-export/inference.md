# [M] Infer instances in one pass before typing

## Goal

The exporter infers every instance a generic item exports at in one pass over the exported
crates' signatures, before it maps any type, so mapping a signature never discovers a new
instance. Two cases that inference misses today export:

- a generic function inferred at an instance that also takes a closure or a foreign
  callback, such as `fn apply<T: Copy, F: Fn(u32) -> u32>(p: Pair<T>, f: F)`;
- a generic type's concrete instance that only an impl of an exported trait names, such
  as `impl Seed<u64> for Wrap<u32>` taken by `fn seeded(seed: impl Seed<u64>)`.

Instance names stay the shortest unique form, independent of crate order.

## Plan

Decided with the maintainer on 2026-10-28, after
[#152](https://github.com/kixelated/rust2/pull/152): restructure inference so these cases
fall out of the design rather than being patched one by one. Write the design note in the
PR body before coding, and check whether rustc's `rustc_monomorphize::collector` or another
analysis can be reused.

Public API: none beyond the instances and functions these cases add.
