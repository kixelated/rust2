# [M] Duration's float methods

## Goal

rust2-ts translates `std::time::Duration`'s float methods (`as_secs_f64`,
`from_secs_f64`, `mul_f64`, `div_f64`, `div_duration_f64`, and their `f32` and `try_`
forms) with std's results, rounding, and panics, proven by differential tests against
native Rust.

## Plan

Decided with the maintainer on 2026-10-27: rust2 aims to support all of std, so generic
gaps are planned without a moq consumer. Nothing in moq-net, moq-clock, or kio uses these.

- Blocked on floats: rust2-ts refuses `f32` and `f64` wherever they appear, so no value
  these methods take or return translates yet. Floats need their own quest first.
- The conversions must match std's rounding exactly, including at `u64::MAX` seconds and
  for values a `f64` cannot hold, which std rejects with a panic or an `Err`.

Public API: the translator's output.
