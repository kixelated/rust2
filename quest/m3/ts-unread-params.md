# [S] A parameter read only where lowering leaves nothing

## Goal

[rust2-ts](/ts/README.md) marks a parameter unused, as `noUnusedParameters` wants, when
lowering drops every read of it, so the output type-checks: an instance of a generic
`Drop` impl over a `T` that drops nothing, such as crossbeam-epoch's own `OnceLock<T>` of
a `u32`, is `function drop$Drop_u32(self: Lock<rt.u32>): void {}`, which `tsc` refuses.

## Plan

Found by [#268](https://github.com/moq-dev/rust2/pull/268) on 2026-10-30. Its `drop` is
`if self.once.is_completed() { (*self.value.get()).assume_init_drop() }`, and with
nothing to drop the `if` lowers to nothing. `analyze` in `ts/src/lower/mir/mod.rs` names
parameters by MIR's reads, before lowering drops the empty branch. moq's collector drops,
so no consumer hits it at the pin.

Public API: the translator's output.
