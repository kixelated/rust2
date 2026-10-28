# [M] bytes' BytesMut, Buf, and BufMut

## Goal

rust2-ts maps `bytes::BytesMut` and the `Buf` and `BufMut` traits with no config, as
[#147](https://github.com/kixelated/rust2/pull/147) maps `Bytes`, so `ts/moq/model.toml`
drops its `BytesMut` binding and moq-net's code generic over `Buf` and `BufMut` translates.
Proven by differential tests against native Rust.

## Plan

Found by #147 on 2026-10-27: the model reaches `BytesMut`, which is bound because its
`unsafe` doesn't translate, and moq-net's codec is generic over `Buf` and `BufMut`.

- Share `Bytes`'s representation where `BytesMut` freezes into one or splits off, and match
  `bytes`' observable behavior: `capacity`, `reserve`'s growth, and what `split_to` and
  `freeze` share.
- `Buf` and `BufMut` as generic bounds belong here; `dyn Buf` belongs to
  [trait objects](/quest/m3/ts-dyn.md).
- Measure the codec on the [TS benchmark](/ts/README.md#bench) before and after.

Public API: the translator's output.
