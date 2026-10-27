# [S] Translate derived == through references and slice index writes

## Goal

rust2-ts translates `<&T as PartialEq>::eq`, which a derived `==` on an enum holding
references reaches, and `live[0] = x` on a `&mut [u8]` parameter, proven by
differential tests against native Rust.

## Plan

Found by [#83](https://github.com/kixelated/rust2/pull/83)'s tests on 2026-10-26.
Both are refused with a span today: the first as unsupported, the second as a raw
pointer.

- Report each one's count in moq-net's census before and after.

Public API: the translator's output.
