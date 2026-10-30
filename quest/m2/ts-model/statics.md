# [M] Statics and constants read from memory

## Goal

rust2-ts translates a `static` of any type, and a `const` with no value tree, by reading
the value rustc evaluated, so `AtomicPtr` statics, std's `OnceLock`, `LazyLock`, and
`LazyCell` statics, and a `const` holding an atomic (lock_api's `RawMutex::INIT`)
translate, along with crate types such as foldhash's and crossbeam's. Each one is proven
by differential tests against native Rust.

## Plan

Decided with the maintainer on 2026-10-29: the general rule, not only the three named
cases. Today a static translates only if it is an atomic of a bool or an integer, or an
immutable `[u8; N]` ([#232](https://github.com/kixelated/rust2/pull/232)). A constant
translates only if it has a value tree, and every other one is refused.

- **Read the evaluated memory.** For a static, use `eval_static_initializer`. For a
  constant with no value tree, use its `ConstValue`. Walk the memory by type, as
  `const_value` already destructures memory for other constants. A part of a type the
  translator refuses is still refused, with its span.
- **Identity follows Rust.**
  - A static is one module value, shared by every use, as #232's atomics are.
  - A `const` is a fresh value at every use, since Rust copies it there. Two uses of
    `RawMutex::INIT` must lock independently.
  - Refuse a `static mut`, as today.
- **Std's cells in memory:**
  - An atomic becomes `rt.Atomic` holding the value read.
  - An `UnsafeCell` is its contents.
  - A `OnceLock` or `OnceCell` becomes the empty runtime cell. Const evaluation can't
    fill one, so refuse any other state.
  - A `LazyLock` or `LazyCell` becomes a new runtime cell holding its init function. The
    first `Deref` or `force` runs the function, exactly once.
  - A `MaybeUninit` is read as #201 models it.
- **Pointers follow [unsafe code](/quest/m3/ts-unsafe.md)'s typed model.**
  - A null `*mut T` constant, as wasmtimer's are, becomes `rt.NULL`.
  - An `AtomicPtr` becomes an `rt.Atomic` holding a raw pointer.
  - `compare_exchange` and `swap` compare pointers as `ptr::eq` does, so two
    `rt.Place`s of one location count as equal.
  - A pointer into memory the translator can't name is refused.
- **Consumer: the moq-net census.**
  - The [model's census](/quest/m2/ts-model/README.md) (2026-10-29) has two groups:
    - 7 statics: foldhash's `GlobalSeedStorage` and crossbeam-epoch's `OnceLock<Collector>`
      and `GuardWrapper`;
    - 3 constants with no value tree: crossbeam-epoch's `Deferred` and moq-net's
      `MaybeUninit<Frame>`.
  - The whole crate adds these, all behind the bound clock:
    - 9 `AtomicPtr` statics and 7 null pointer constants, in parking_lot_core and
      wasmtimer;
    - 3 `OnceLock` and `LazyLock` statics, in moq-clock and moq-net's `clock.rs`;
    - lock_api's `RawMutex::INIT`.
  - Report the census delta for both. hashbrown's `RawTableInner` constants go to
    [hashbrown](/quest/m2/ts-model/hashbrown.md). The waker vtables go to
    [trait objects](/quest/m3/ts-dyn.md). wasm-bindgen's statics belong to the platform.
- **Differential tests** in `ts/fixtures/lang/src/statics.rs` cover:
  - a static of a struct with an atomic field;
  - two uses of one atomic-holding `const`;
  - a `LazyLock` static forced from two call paths;
  - an empty `OnceLock` static;
  - an `AtomicPtr` static swapped between null and a boxed value.

Public API: the translator's output.
