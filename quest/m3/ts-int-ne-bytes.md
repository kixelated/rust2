# [S] Integers' little- and native-endian bytes

## Goal

[rust2-ts](/ts/README.md) translates each integer type's `from_le_bytes`, `to_ne_bytes`,
and `from_ne_bytes` as it does `from_be_bytes`, `to_be_bytes`, and `to_le_bytes`, proven
by differential tests against native wasm32 at every width.

## Plan

Found while writing [#296](https://github.com/moq-dev/rust2/pull/296)'s fixture, which
used `to_le_bytes` instead. Planned without a moq consumer, under the maintainer's
2026-10-27 decision that rust2 aims to support all of std.

Reproduction at `cecbf0d7`, a scratch crate whose roots call each:

```text
unsupported call to `core::num::<impl u32>::from_le_bytes` at src/lib.rs:2:2
unsupported call to `core::num::<impl u64>::to_ne_bytes` at src/lib.rs:6:23
unsupported call to `core::num::<impl u64>::from_ne_bytes` at src/lib.rs:6:2
```

- `library.rs` maps integers' `from_be_bytes`, `to_be_bytes`, and `to_le_bytes`, the last
  as the big-endian bytes reversed. wasm32 is little-endian, so `_ne_` is `_le_`, as
  `float.rs` already maps a float's `to_ne_bytes` and `from_ne_bytes`.
- `from_le_bytes` must not reverse its argument in place: the caller may still hold that
  array. Reverse a copy, or give each width a runtime `fromLeBytes`.
- Test every width, the 64- and 128-bit ones past 2^53 included, in the lang fixture's
  `ints` cases.

Public API: the translator's output.

## Related

- [Unsafe code the translator can model](/quest/m3/ts-unsafe.md) - `bytes`' getters also need each integer's `from_le_bytes` and `from_ne_bytes`
