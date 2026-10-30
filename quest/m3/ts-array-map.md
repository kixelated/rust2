# [S] An array's map

## Goal

`<[T; N]>::map` translates for every element type, its closure called on each element in
order, proven by a differential test against native Rust that covers `[u8; N]` results.

## Plan

Reproduced on `955ac19f`: `[bool; 3].map(u8::from)` is refused at its span as an
unsupported call to `std::array::<impl [bool; 3]>::map`. A `[u8; N]` result is a
`Uint8Array`, as `array::from_fn`'s is.

Found by [#302](https://github.com/moq-dev/rust2/pull/302).

Public API: the translator's output.
