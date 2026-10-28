# [S] Payloads cross as bytes

## Goal

A parameter that takes bytes, such as moq's `impl Into<Bytes>` frame payloads
(`group::Producer::write_frame`), exports as bytes in the C ABI and every generator, so a
payload that is not valid UTF-8 crosses unchanged. Today it resolves to `String`: C and C++
abort on such a payload, and Kotlin's `writeFrame` takes a `String`.

## Plan

Found by [#184](https://github.com/kixelated/rust2/pull/184) on 2026-10-28. This is a
correctness bug: moq's media payloads are binary.

- Find why first. [`choose`](/ffi/src/collect.rs) puts `X` of `impl Into<X>` ahead of
  `String`, so the signature moq writes, or the bound typing reads, differs from what the
  rule expects.
- Fix the rule, not moq's case: any parameter whose bounds `Bytes` or `Vec<u8>` meets but
  whose meaning is bytes must not narrow to `String`. A `String` choice stays only where
  the bound itself is about text.
- Prove it from C, C++, and Kotlin with a payload that is not UTF-8, and report the census
  before and after.

Public API: the C, C++, and Kotlin types of the affected parameters.
