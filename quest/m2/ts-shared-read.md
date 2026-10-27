# [S] Share what the crate never changes in place

## Goal

rust2-ts shares a `Copy` aggregate, where it copies one today, when nothing in the crate
writes into a value of that type in place. `Decoder::varint`'s `leading-ones` decode
on `just ts bench` is back to its speed before
[#83](https://github.com/kixelated/rust2/pull/83) in Chrome, and every differential
test still passes.

## Plan

Decided with the maintainer on 2026-10-27, replacing the narrower fix for
`self.form`'s copy that this quest held before:
[#83](https://github.com/kixelated/rust2/pull/83) and
[#99](https://github.com/kixelated/rust2/pull/99) copy a `Copy` aggregate before a call
can share it (`self.form` before `read`, `copy$Form` in `Decoder::new` and
`Encoder::new`, and map keys). A copy is only observable if something writes into a
shared value in place: a field or element assignment through it, or a `&mut` to its
inside. A type the crate never writes that way can be shared freely.

- Find, per crate, each `Copy` aggregate type that some place writes into in place, and
  let `copy_fn`'s callers skip the copy for every other type.
- Keep the analysis whole-crate and fail closed: a write the analysis can't see, such as
  through a generic, counts as a write.
- Prove it with #83's argument-aliasing cases and #99's store cases, plus a new case
  where the only in-place write is in another module or behind a generic.
- Report the copies removed in moq-net's output, and the Chrome codec bench before and
  after.

Public API: the translator's output.
