# [M] Closures lent for the call

## Goal

The exporter exports a function that takes a closure it calls only during the call, as a
C callback with a context pointer, so moq's `catalog::Producer` can be changed through a
lent closure (`mutate`) where `modify` returns a borrowing `Guard` the C ABI can't hold.

## Plan

Left out of [#126](https://github.com/kixelated/rust2/pull/126); decided with the
maintainer on 2026-10-27 to plan it.

- A closure the callee keeps past the call stays refused; only a lent one exports.
- A panic or an error in the callback must not unwind across the C ABI.
- Decide whether `mutate` is moq's API or the exporter's rewrite of `modify`; moq's own
  change belongs in moq's tree.

Public API: the exporter's C ABI for callbacks.
