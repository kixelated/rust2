# [L] Unsafe code the translator can model

## Goal

[rust2-ts](/ts/README.md) translates the `unsafe` operations it can give exact semantics in
JS, so a dependency using only those translates from source instead of being bound by
hand. What is left: pointers into a slice or `Vec`, `ManuallyDrop`, unions over a type
parameter, and a raw pointer to a value JS copies taken through a reference.

## Plan

Decided with the maintainer on 2026-10-27: a dependency containing `unsafe` is bound by
the consumer [at first](/ts/README.md#bindings), and moq gives its own `unsafe` safe
`cfg(rust2_ts)` twins. Both are stopgaps, since a stock crate should translate with no
annotations or config; this quest is what retires them.

Decided with the maintainer on 2026-10-28, typed pointers: a raw pointer is what its
reference is, the object, an `rt.Place`, or `rt.NULL`, and a pointer into a slice or `Vec`
is `{ array, index }`. Byte reinterpretation, pointer-to-integer casts, and raw allocation
are refused with a span. [#201](https://github.com/moq-dev/rust2/pull/201) landed `Box`
to raw and back, `NonNull`, `MaybeUninit`, raw reads and writes, `get_unchecked`, and
unions whose overlapping fields share a type, which took hashlink from 375 refusals to
none of its own `unsafe`.

- An `unsafe` operation outside the modeled set is refused with its span, as today.
- Undefined behavior in Rust need not be reproduced, but a translated program never
  silently differs from native where Rust's behavior is defined.
- Element pointers, `{ array, index }`, with `add`, `offset`, `offset_from`, and
  `slice::from_raw_parts` over them: smallvec, arrayvec, and moq-net's `frame.rs`.
- `ManuallyDrop<T>` as `T` with its drop skipped, as `MaybeUninit` is: crossbeam-epoch's
  `Local` reaches it, though crossbeam's tagged pointers stay refused.
- A union over a type parameter, whose layout has no fixed offsets until an instance
  fixes them (smallvec's `SmallVecData`): lay out each instance instead of the identity.
- `&raw const *r` where `r` is a `&T` to a value JS copies or a library object, such as
  foldhash's seed and arrayvec's `as_ptr`: the reference is the value, not where it is
  kept, so it needs the place the reference came from.
- `UnsafeCell::raw_get`, `from_mut`, and `replace`, refused with their span: none is in
  moq's census at the pin.
- A slice's pointer cast to an array's of its length, `*(s as *const [u8] as *const
  [u8; N])`, which `bytes`' `Buf::get_u16` and wider read a crate's own buffer through
  ([#222](https://github.com/moq-dev/rust2/pull/222)): the bytes keep their type, so
  this reinterprets nothing. Those getters also need `<[u8]>::get` and `get_mut` of a
  range, and each integer's `from_le_bytes` and `from_ne_bytes`, and `chunks_vectored`
  needs `std::io::IoSlice`: all 184 of moq-net's refusals in `bytes`.
- `BinaryHeap` and `PeekMut` translate from std's MIR, whose `Hole` moves elements
  through raw pointers into the `Vec`'s storage, replacing `rt.BinaryHeap` and
  `rt.PeekMut`, the hand-written port of std's sifts that
  [#252](https://github.com/moq-dev/rust2/pull/252) mapped them to and that must track
  std's by hand. The lang fixture's `orders` differential case proves the translation
  still matches std's comparisons, pop order, and drops.
- `Vec::drain` translates from std's MIR, whose `Drain` reads its range out and moves the
  tail back through raw pointers into the `Vec`'s storage, replacing `rt.vecDrain` and
  `rt.byteVecDrain`, the runtime port that
  [the slice gaps' PR](https://github.com/moq-dev/rust2/pull/259) mapped it to. The lang
  fixture's `slices::drains` differential case proves the translation still matches std's
  items, drops, and what a forgotten drain leaves.

Public API: the translator's output.
