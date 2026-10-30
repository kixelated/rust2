# [XS] An array as a slice

## Goal

rust2-ts translates `<[T; N]>::as_slice` and `as_mut_slice`, for bytes and other values.
Proven by differential tests against native Rust.

## Plan

`as_slice` refuses with a span today, as "unsupported call to `std::array::<impl [u8; 2]>::as_slice`".
A `&[u8; N]` is already an `rt.ByteSlice` and a reference to another array its JS array,
so each is the reference it is given.

Public API: the translator's output.
