# [S] Iterator skips that read as std's

## Goal

rust2-ts's `nth`, `nth_back`, and the adaptors built on them read the same items std's
do of an iterator that drops nothing, so a `by_ref` source or an `inspect` closure sees
std's reads, proven by differential tests against native Rust.

## Plan

Found by review of [#101](https://github.com/kixelated/rust2/pull/101) on 2026-10-27.
Without a dropper, `rt.iterNth` and `nthBackItem` are `main`'s methods, which only
`Enumerate`, `Rev`, `Take`, `Range`, and the `by_ref` wrapper override. std's `Fuse`,
`Skip`, `Chain`, `StepBy`, `Peekable`, `Flatten`, and `Copied` forward to
what they wrap, and `Take`'s `nth_back` and `advance_back_by` read past its limit, so
`(0..3).by_ref().take(0).fuse().nth_back(0)` leaves the range empty in Rust but not here.
#101's `rt.owned*` dispatchers already follow std's order where an item drops.

- Weigh calling those dispatchers without a dropper against methods on each class: the
  codec's bundle carries every class a dispatcher names.
- Cover `by_ref` and `inspect` sources under each adaptor.

Public API: the translator's output.
