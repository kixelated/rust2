# [M] Share constant aggregates

## Goal

rust2-ts lowers a constant aggregate nothing can write into, such as `Ok(())`, to one shared
frozen object rather than a new one per evaluation, so a successful write in moq-net's
encoder allocates no `Result`. `just ts bench`'s encode workloads are faster than A/A's
spread in Chrome, and every differential test still passes.

## Plan

Found by the loop-invariant aggregate spike on 2026-10-27: the engines already remove the
aggregates translated loops rebuild, so hoisting them was dropped, but not a constant
returned through a call they don't inline, such as `Encoder::varint`'s and
`varint::write`'s `Ok(())`. Sharing it by hand cut Node's scavenges on 1-byte varint
encode by 95%, and measured 0.96x `translated` on Chrome's encode workloads over three runs
(A/A 0.99-1.05), 0.90-0.93x in Firefox, 0.93x in Bun, and 0.94-0.97x in Node, with the rest
within noise. Freezing it cost nothing measurable.

Decided with the maintainer on 2026-10-27: all constant aggregates, not only `Ok(())`.

- `Ok(())` and `Continue(())` need no analysis: the payload is `()`, and a store through
  `&mut Result` rebinds the place.
- Other constants (`Err(DecodeError::Short)`, fieldless variants, limb literals, `const`
  arrays, `rt.fmtSpec` objects) share only when nothing in the crate writes into their type
  in place, the analysis that already shares `Copy` aggregates
  ([output](/ts/README.md#output)); a `ref mut` to a payload, as in
  `Result<Option<T>, E>`, keeps one object per evaluation. Prove each with such a case.
- Freeze each shared object, so a write the analysis missed throws in strict mode.
- Measure each kind: hoisting `{ tag: "Short" }` and a `u64` limb saved no allocation in
  V8, so keep only the kinds that measure.
- Update the README's claim that making no `Ok` objects measured no faster, measured in
  [#15](https://github.com/kixelated/rust2/pull/15) under THIR.

Public API: the translator's output; callers receive the same frozen object for a constant.

