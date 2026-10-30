# [L] Writes judged by the instances translation reaches

## Goal

[rust2-ts](/ts/README.md) judges whether the crate may put another object in place of a
part kept as an object by the instances translation reaches, not by every type a
generic body's bounds allow, so crossbeam-epoch's own `OnceLock<Collector>` static and
the `&raw` pointers to fields that nothing replaces translate, proven by differential
tests against native Rust, while a pointer to a part some reached instance does replace
stays refused with its span.

## Plan

Split from [m3/ts-unsafe](/quest/m3/ts-unsafe.md) by
[#268](https://github.com/moq-dev/rust2/pull/268) on 2026-10-30, which translates std's
`UnsafeCell` and refuses the pointer `UnsafeCell::get` gives as `out_of_step` refuses a
`&raw`. `Writes` in `ts/src/lower/copy.rs` reads each body over its own type parameters,
so a generic body writing a whole `T` counts for every `T` its bounds allow. In the
model's census, these each count for every type, `OnceLock<Collector>` included:

- crossbeam-epoch: `OnceLock::<T>::initialize`'s closure and `primitive::cell::UnsafeCell::<T>::with_mut`
- hashbrown: `HashMap::insert`, `raw::Bucket::<T>::as_ptr`, `from_base_index`, and
  `write`, and `ScopeGuard`'s `Drop`
- hashlink: `RawOccupiedEntryMut::replace_value`
- kio: `log::Writer::<T>::push`'s closure and `sync::UnsafeCell::<T>::with_mut`
- smallvec: `SmallVec::<A>::truncate` and `Drain`'s `Drop`, over `<A as Array>::Item`

Binding smallvec alone leaves the rest. At the pin, after #268, `get` refused only by
this is 7 of the model's 416 refusals (`once_lock.rs:64` and `:77`, crossbeam's wrapper
at `lib.rs:116` and `:121`, kio's `sync.rs:42`, foldhash's `seed.rs:236`), 36 of
moq-net's 1463, and 4 of kio's 136; the `&raw` pointers to fields refused the same way
come on top.

- Record each generic write with the body that made it, and check it against the
  arguments of the instances translation reaches, as it already queues them; a census
  lowered over type parameters keeps today's judgment.
- Drop the bounds the lang fixture puts on its copies of crossbeam's `OnceLock` (`Locked`)
  and wrapper (`Lent`), which exist only to dodge this.

Public API: the translator's output; fewer refusals.
