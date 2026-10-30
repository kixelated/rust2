# [M] Byte slices returned

## Goal

A translated function that returns a value built of byte slices and scalars, such as
`Result<(&[u8], T), E>`, hands its parts to the caller without an object, and Chrome's
varint and frame decode on `just ts bench` are faster than with slices passed as parts
alone, with every differential test passing.

## Plan

The [epic's](/quest/m2/ts-byte-views/README.md) hand edit measured Chrome 1.84x on varint
decode and 1.65x on frames with locals and returns, against [#120](https://github.com/moq-dev/rust2/pull/120).

Decided with the maintainer on 2026-10-27, from
[#138](https://github.com/moq-dev/rust2/pull/138): remeasure after
[arguments as parts](/quest/m2/ts-byte-views/arguments.md). #138 built a per-function
scratch object (the function returns which variant it made and writes the parts into its
module-level scratch), and it measured slower than `main` in Chrome (varint quic/1B 12.0
to 15.2 ns, frames 3B-reads 152 to 208 ns). One shared scratch per return type, reading
only the branch taken, measured about even; module `let`s measured 0.93x. Its branch keeps
the code. Pick the ABI only if it beats arguments alone.

- Only functions outside the API boundary that are never used as a value: not roots, fn
  pointers, or closures.
- The callee writes its scratch at the JS `return`, after its MIR drops ran, so a `Drop`
  that re-enters cannot overwrite it; the caller reads it at once, and temporary folding
  never delays that read.

Public API: the translator's output for byte slices.

## Required

- [Byte slices passed as parts](/quest/m2/ts-byte-views/arguments.md) - the measured win, landed first
