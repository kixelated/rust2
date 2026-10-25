# [M] C-first bindings spike

## Goal

A prototype of a C-first layering in [the fork](/uniffi/README.md)'s generators:
the C API sits directly on UniFFI's `extern "C"` ABI with no C++ underneath, and
the C++ output is generated per standard (for example C++17, 20, and 23) on top
of that one C ABI. It covers a slice of moq-ffi with an async call, an error,
bytes, and a record, and is compared with today's C-over-C++ on call overhead,
generated size, and the compile matrix (GCC, Clang, and MSVC, with and without
exceptions and RTTI). Its report plans the full rewrite.

## Plan

Decided with the maintainer on 2026-10-24, after
[kixelated/rust2#34](https://github.com/kixelated/rust2/pull/34) left the C++
generator with two error styles that each support only part of UniFFI:

- C-first replaces today's layering, where the C backend wraps expected-style C++.
- The generated C replaces moq's hand-written cbindgen `rs/moq-c`; moq's quest for
  that follows this spike.
- Callback interfaces are added at the C level, which subsumes giving the
  expected style callback interfaces.
- The exceptions style goes away with the old C++ layer.
- Per-standard C++ output was chosen over one C++17 header with feature tests.
  C++ types may differ per standard, but every variant links against the same C
  ABI. The C++ layer may be headers or compiled `.cpp` files built per standard;
  pick whichever measures better and record why.
- Both outputs take a `--std` flag that opts into newer output, defaulting to the
  oldest standard (C11, C++17):
  - C: `c11` (today's floor, for anonymous unions), `c17`, or `c23`. C23 can opt
    into `bool` and `nullptr`, typed enums (`enum : uint8_t`), `[[nodiscard]]` on
    fallible calls, and `constexpr` constants.
  - C++: `17`, `20`, or `23` selects the per-standard output.
  - The C ABI is identical across `--std` values, so one library build serves
    every consumer; only declarations and annotations change. Measure what each
    opt-in buys and drop any that changes nothing measurable.
- OBS builds at C17 and C++17 with exceptions and RTTI on, and its plugin API is
  C (obs-studio's `cmake/common/compiler_common.cmake`).
- The C name clash moq-ffi hits (`moq_audio_encoder_finish` emitted twice) is
  fixed in the generator's naming rule.
- moq's expected-style C++ bindings are today's baseline; the rewrite may change
  them, so the report says how moq migrates.

## Required

- [Waiting: the exporter decision](/quest/m1/ffi-export-decision.md) - paused until the maintainer decides whether this folds into the exporter, as the spike recommends

## Related

- [Nightly MSVC and macOS](/quest/m2/uniffi-ci-msvc-macos.md) - the compile matrix this spike measures by hand, run nightly
