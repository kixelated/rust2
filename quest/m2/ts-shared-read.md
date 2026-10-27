# [XS] Share a held-back read across a call

## Goal

`Decoder::varint`'s `leading-ones` decode on `just ts bench` is back to its speed before
[#83](https://github.com/kixelated/rust2/pull/83) in Chrome, with every differential
test still passing.

## Plan

Decided with the maintainer on 2026-10-26: #83 fixed a silent copy miscompile and
accepted a 2-19% `leading-ones` decode cost in Node and Chromium to do it. The
translator now copies `self.form` before calling `read`, because an earlier pass stops
holding back the read of `self.form` before that call, so the call can't share it.

- Let the held-back read reach the call when nothing between them can change the
  source, so `self.form` is shared again without a copy.
- Prove the read is still exact with the argument-aliasing cases #83 added.

Public API: the translator's output.
