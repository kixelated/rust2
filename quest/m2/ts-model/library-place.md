# [M] A write through a mutable reference to a library object

## Goal

A whole write through a `&mut` to a library object, such as a `Vec`, an `Arc`, or a
`BTreeSet`, writes the slot the reference points at, wherever the reference came from,
proven by differential tests against native Rust:

- `*r = v` where `r` came out of an `Option`, as in
  `if let Some(r) = v.get_mut(0) { *r = Arc::new(1) }`, which translates today but
  silently differs from native.
- The replacements refused today: `mem::take(v)` and `mem::replace` with `v: &mut Vec<_>`,
  and `*r = v` through a `MutexGuard`'s deref or a map lookup.

## Plan

Found by [#201](https://github.com/kixelated/rust2/pull/201) on 2026-10-28, in safe code:
the translation assigns the local the `Option` was matched into, so the `Vec` keeps its old
`Arc`, whose count stays up. It silently differs from native, so it is fixed first.

Both have one root: a `&mut` to a library object is the object, which a write can't put
another object in place of, and so is one to the crate's own struct or enum. On main, at
moq `30f25c04`, moq-net's census refuses eight such sites, after
[#204](https://github.com/kixelated/rust2/pull/204) translated the write through a
closure's capture:

- "replacing the value behind a reference": a `Vec` at `ietf/publisher.rs:3794` (through a
  `MutexGuard`), `lite/publisher.rs:4043`, and `model/broadcast.rs:1226`; a `BTreeSet` at
  `lite/subscriber.rs:7620`; and the crate's own `Subscription` at
  `ietf/publisher.rs:16954`, `TrackState` at `ietf/subscriber/tracks.rs:1118`, and `Parked`
  at `model/expiry.rs:316`.
- "an assignment through a reference": an `Arc` at `model/frame.rs:752`.

- A whole write through such a reference goes through the `rt.Place` of where the object
  is kept, as a `&mut` to a value JS copies does, or into the object in place where the
  runtime type allows it. Where neither is known, the write stays refused with its span.

Public API: the translator's output.
