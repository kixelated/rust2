# [M] Writes into fresh memory before any pointer to it

## Goal

[rust2-ts](/ts/README.md) stops counting a whole write that fills memory nothing points
into yet, such as a slot of a fresh allocation or a `MaybeUninit` written before any
pointer is taken, as putting another object in place of a part kept as an object. A raw
pointer taken after such writes then translates. A write that may run while a pointer
into the memory exists stays refused with its span, proven by differential tests against
native Rust.

## Plan

Planned on 2026-10-30 as a follow-up of
[#281](https://github.com/moq-dev/rust2/pull/281), which judges writes by the instances
translation reaches. `Writes` in `ts/src/lower/copy.rs` is flow-insensitive: any reached
whole write of a type marks the type replaced, wherever and whenever it runs.

Reproduction: `just ts census` at moq's pin (`30f25c04`) and rust2's `c01ea740`, in
the model's census (`ts/moq/model.toml`):

- `rs/kio/src/sync.rs:42:12: a raw pointer to std::cell::UnsafeCell<std::mem::MaybeUninit<model::group::Entry>> in kio::sync::UnsafeCell<std::mem::MaybeUninit<model::group::Entry>>, which the crate may put another object in place of`
- the same at `sync.rs:46:12`, through `with` and `with_mut`

`kio::log::Segment::<Entry>::new` is the only writer. It fills each slot of a fresh
`alloc::alloc` with `ptr::write(UnsafeCell::new(MaybeUninit::uninit()))` before handing out
any pointer, and later writes go through `with_mut` into the `MaybeUninit`, not the cell.
moq-net's census reports the same two lines.

The quest sits in m3, not m2: the model's census reaches these lines, but the same
`Segment::new` is refused at `log.rs:90` for `std::alloc::alloc`, and
[m3/ts-unsafe](/quest/m3/ts-unsafe.md) decides that raw allocation stays refused. So
fixing this alone translates no more of the model. kio's log translates once moq gives it
a safe `cfg` twin, as [m2/ts-model](/quest/m2/ts-model/README.md) plans, or raw allocation
is modeled. Recheck which of those holds first, since a safe twin may never write a slot
whole.

- Judge such a write by the memory it fills, not by its type everywhere: for example, a
  write through a pointer derived from a fresh allocation or a `MaybeUninit`, made before
  that pointer or one derived from it escapes. Prefer a rustc dataflow analysis over a
  hand-written one, as `holders` in `ts/src/lower/mir/holders.rs` bounds a pointer by
  `MaybeLiveLocals`.
- A write after the pointer escapes, or one the analysis can't place, counts as today.
- Measure the model, moq-net, and kio census before and after.

Public API: the translator's output.
