# [L] Floats

## Goal

[rust2-ts](/ts/README.md) translates `f64`: its arithmetic, comparisons, `as` casts to and
from every integer width, its methods and constants, `str::parse`, its formatting, and
`Duration`'s `f64` methods, proven by differential tests against native Rust. moq-net's
`Bandwidth::scaled`, its probe thresholds, and its `Duration::mul_f64` jitter translate.

## Plan

Decided with the maintainer on 2026-10-27, from
[#135](https://github.com/kixelated/rust2/pull/135):

- `f64` here, since moq-net uses it; [`f32`](/quest/m3/ts-f32.md) follows in m3.
- An `f64` is a JS number. Every operation std specifies exactly matches native bit for
  bit, including where JS differs: `round` rounds half away from zero, `max` and `min`
  ignore a NaN, a cast to an integer saturates and sends NaN to 0, and a 64- or 128-bit
  integer rounds to the nearest `f64` once. `str::parse` follows std's grammar, not
  `Number`'s.
- The functions std leaves to the platform's libm (`sin`, `exp`, `ln`, `powf`, `powi`, and
  the rest) map to `Math.*`. Their tests compare within a small ULP bound the README
  states, not an exact libm port. `mul_add` is specified as one rounding, which `Math`
  has no cheap form of, so it is refused with a span until a consumer needs it.
- `to_bits` and `from_bits` go through a `DataView`. JS does not guarantee a NaN's
  payload, so the tests cover payloads in Node and Chrome and the README notes it.
- This quest owns float formatting byte for byte as `core::fmt`: `Display`, `Debug`
  (which switches to an exponent), a precision, and `{:e}`/`{:E}`.
  The other types' format specs already translate.
- `Duration`'s `f64` methods (`as_secs_f64`, `from_secs_f64`, `mul_f64`, `div_f64`,
  `div_duration_f64`, and the `try_` forms) port std's integer algorithms, with its
  panics and `Err`s at `u64::MAX` seconds, negative values, and NaN.
- Report the model's census before and after.

Public API: the translator's output.
