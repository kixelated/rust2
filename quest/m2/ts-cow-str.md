# [M] Copy-on-write strings

## Goal

rust2-ts translates `Cow<str>`, so moq-net's `ietf/namespace.rs` translates with no
refusals, proven by differential tests against native Rust.

## Plan

Found by [#96](https://github.com/kixelated/rust2/pull/96) on 2026-10-27: after `str`'s
char methods translate, `<Cow<str> as From<&str>>::from` is `namespace.rs`'s last
refusal, and `Cow<str>` accounts for 139 of moq-net's.

- Choose the representation with the maintainer: a plain JS string, since `str` is
  immutable in the output, or a tagged value that keeps `Cow::Borrowed` and
  `Cow::Owned` apart for `matches!` and `is_borrowed`.
- Report moq-net's census before and after.

Public API: the translator's output.
