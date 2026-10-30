# [M] A mutable reference to a reference

## Goal

[rust2-ts](/ts/README.md) translates a `&mut` to a reference, such as `&mut &[u8]` and
`&mut &mut T`, proven by differential tests against native Rust. Then `bytes`' `Buf` of a
`&[u8]` advances, the `Buf` and `BufMut` impls forwarding through `&mut T` translate from
`bytes`' MIR, and so does `Buf::copy_to_bytes` of a crate's own buffer, which reads it
through a `Take<&mut Self>`.

## Plan

Found by [#222](https://github.com/moq-dev/rust2/pull/222) on 2026-10-29: a `&mut` to a
type JS copies is an `rt.Place`, but a reference is never one, so `&mut &[u8]` is refused
where it is made. moq-net's census refuses `<&mut &[u8] as Buf>::remaining` and
`has_remaining` and `<&[u8] as Buf>::copy_to_bytes` in `transport/memory.rs`, which writes
a `&[u8]` through `poll_write_buf<B: Buf>`.

- Writing a whole reference through it, `*r = &r[n..]`, is what `<&[u8] as Buf>::advance`
  does, so a place of the reference is the likely shape, as for a value JS copies.
- Where only what the inner reference points at is read or written, as the forwarding
  impls do, the outer reference could be the inner one's value instead. Measure whether
  the codec's decode on the [TS benchmark](/ts/README.md#bench) cares.

Public API: the translator's output.
