# [S] Stable names in the TypeScript translator

## Goal

A caller's translated names never change when unrelated code adds a generic
instance or a trait impl. Every generic instance always prints its type-argument
suffix (`foo$u8`, `wrapGet$Wrap_Pair`), and every trait-impl function always prints its
trait (`pairFmt$Display`), even when it is the only one.

## Plan

Decided with the maintainer on 2026-10-24: always suffix, in today's Rust-like
suffix format, not a hash. Today a generic function with one instance is `foo`
until a root reaches a second instance, and a type with only `Debug` gets `fmt`
until it also has `Display`; either change renames calls in code the new root
never reaches. Found in [#18](https://github.com/kixelated/rust2/pull/18) and
[#27](https://github.com/kixelated/rust2/pull/27), which notes that numbered `$2`
suffixes can shift the same way.

Public API: the translator's output names for generic instances and trait-impl
methods.
