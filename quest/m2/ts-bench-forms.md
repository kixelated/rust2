# [S] The bench's varint forms at moq's pin

## Goal

`just ts bench` runs to its report at the moq pin, its hand-written route decoding each
varint form as the codec fixture encodes it.

## Plan

Found by [#222](https://github.com/kixelated/rust2/pull/222) on 2026-10-29: #196's pin
dropped `Ietf.DRAFT_16` and `DRAFT_17` from moq's `js/net`, so `bench/moq.ts`'s `FORMS`
passes `undefined` as both versions. moq's `isLeadingOnes` takes any version but lite's
first six as leading ones, so the hand-written decode of `varint/quic/2B` and wider reads
QUIC wire as leading ones and fails the output check. Bun runs `bench/` untyped, so
nothing caught it.

- Pick each form's version from those moq still has: `Lite.DRAFT_06` for QUIC, and
  `Lite.DRAFT_07` or `Ietf.DRAFT_18` for leading ones. #222 measured with `Lite.DRAFT_06`.
- Type-check `bench/` in `just check`, so a pin bump that drops a version fails there.

Public API: none.
