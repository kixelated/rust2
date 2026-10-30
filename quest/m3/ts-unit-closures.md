# [S] A closure's () result as a value

## Goal

A closure returning `()` whose result is a value, such as the items of
`data.iter().map(|_| ()).collect::<Vec<()>>()`, translates to TypeScript that `tsc`
accepts, proven by a differential test against native Rust.

## Plan

Reproduced on `dadd60a9`: with the result bound and read twice, the lowering is
`const units: undefined[] = rt.iterToArray(rt.iterMap(..., (_2: rt.u8): void => {}))`,
which `tsc` refuses: `Type 'void[]' is not assignable to type 'undefined[]'`. `fn_ty`
and `ret_ty` in `ts/src/lower/mod.rs` type every `()` return as `void`, so `iterMap`
infers its item as `void`.

Recommended: type a closure's `()` return as `undefined`, which the dev shell's `tsc`
(7.0) accepts on a function with no `return`, and which still satisfies a parameter
typed `=> void`. Keep `void` elsewhere, where only the effect is read, to avoid
churning every function's output. Cover `Vec<()>`, `VecDeque<()>`, and an `Option<()>`
a closure returns, in the lang fixture's `nested`.

Found by [#292](https://github.com/moq-dev/rust2/pull/292), whose fixture builds its
units with `vec![(); n]` to avoid this.

Public API: the translator's output.
