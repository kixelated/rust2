# [S] str and String methods over char

## Goal

rust2-ts translates `str::chars`, the `str` methods that take a `char` pattern,
`char::is_ascii_digit`, and `String::push` through `&mut String`, so moq-net's
`ietf/namespace.rs` translates, proven by differential tests against native Rust.

## Plan

Found by [#86](https://github.com/kixelated/rust2/pull/86)'s census on 2026-10-26,
after `char` values themselves translate. `String::push` through `&mut String` is the
one that blocks `ietf/namespace.rs`.

- Cover multi-byte and surrogate-boundary characters in the differential tests.
- Report moq-net's census before and after.

Public API: the translator's output.
