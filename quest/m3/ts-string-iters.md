# [S] Strings from and into iterators

## Goal

[rust2-ts](/ts/README.md) translates `collect::<String>`, `String`'s `Extend` of chars and
strings, and `String::drain`, proven by differential tests against native Rust.

## Plan

Found by [#271](https://github.com/moq-dev/rust2/pull/271) on 2026-10-30, which translates
`String`'s edits that take and give plain values. Collecting chars into a `String` is
refused too. moq-net and kio call none of these, so this is m3 under the all-of-std
decision.

- Each reads its iterator as std does: `Extend<char>` asks `size_hint` first, which an own
  iterator may observe.
- A `Drain` is an iterator of the drained chars; the `&mut String` it holds keeps anything
  from seeing the string before it drops, so it can be edited at once.

Public API: the translator's output.
