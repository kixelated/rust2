# [M] Byte slices returned

## Goal

A translated function that returns a value built of byte slices and scalars, such as
`Result<(&[u8], T), E>`, hands its parts to the caller without an object, and Chrome's
varint and frame decode on `just ts bench` are faster than with slices held as parts in
locals alone, with every differential test passing.

## Plan

The [epic's](/quest/m2/ts-byte-views/README.md) hand edit measured Chrome 1.84x on varint
decode and 1.65x on frames with locals and returns, against [#120](https://github.com/kixelated/rust2/pull/120).

Decided with the maintainer on 2026-10-27: a per-function scratch object. The function
returns which variant it made, apart from any payload, and writes the parts into its
scratch; an error payload can itself be `undefined` (`Err(())`, `Err(None)`), so it never
doubles as the success signal. One shared `rt` scratch risks shape conflicts between call
sites, and module `let`s measured 0.93x in Chrome.

- Only functions outside the API boundary that are never used as a value: not roots, fn
  pointers, or closures.
- The callee writes its scratch at the JS `return`, after its MIR drops ran, so a `Drop`
  that re-enters cannot overwrite it; the caller reads it at once, and temporary folding
  never delays that read.

Public API: the translator's output for byte slices.
