# [M] Float formatting

## Goal

[rust2-ts](/ts/README.md) formats and parses `f64` as std does, proven by differential
tests against native Rust: `Display`, `Debug` (which switches to an exponent), a precision,
and `{:e}`/`{:E}`, byte for byte as `core::fmt`, and `str::parse::<f64>` by std's grammar,
not `Number`'s. `f64::clamp`, whose panic prints its bounds with `Debug`, translates with
them.

## Plan

Decided with the maintainer on 2026-10-27, from
[#135](https://github.com/moq-dev/rust2/pull/135): this quest owns float formatting; the
other types' format specs already translate. The rest of `f64`, its arithmetic, casts,
methods, and `Duration`'s `f64` methods, landed in
[#165](https://github.com/moq-dev/rust2/pull/165).

- `Display` and `Debug` without a precision print the shortest digits that read back as
  the value. JS's `Number.prototype.toString` picks the same digits, which leaves only the
  layout, `1e-7` against `0.0000001`, to port.
- A precision rounds the exact decimal value half to even, where `toFixed` rounds half up,
  so it needs the exact expansion rather than `toFixed`.
- `str::parse` takes std's grammar (`inf`, `NaN`, no hex, no `_`) and its errors.

Public API: the translator's output.
