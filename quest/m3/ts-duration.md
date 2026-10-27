# [M] The rest of Duration

## Goal

rust2-ts translates the rest of `std::time::Duration`'s API that
[#125](https://github.com/kixelated/rust2/pull/125) refuses, with std's results, rounding,
and panics, proven by differential tests against native Rust: compound assignment
(`+=`, `-=`, `*=`, `/=`) on a place, the float methods (`as_secs_f64`, `from_secs_f64`,
`mul_f64`, `div_duration_f64`, and their `f32` and `try_` forms), `from_mins`,
`from_hours`, `from_nanos_u128`, and `Sum`.

## Plan

Decided with the maintainer on 2026-10-27: rust2 aims to support all of std, so generic
gaps are planned without a moq consumer. Nothing in moq-net, moq-clock, or kio uses these
on a `Duration` itself today; moq-clock's `Instant += Duration` is its own `AddAssign`,
which translates from source with [crates](/quest/m2/ts-model/crates.md).

- `rt.Duration` is immutable, so compound assignment assigns a new value to the place, as
  `String::push` does for a JS string.
- The float conversions must match std's rounding exactly, including at `u64::MAX`
  seconds and for values a `f64` cannot hold, which std rejects with a panic or an `Err`.

Public API: the translator's output.
