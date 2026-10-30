# [M] Zero-sized values made out of sight

## Goal

rust2-ts translates the zero-sized values MIR never assigns, proven by differential tests
against native Rust: a function or closure returning a unit struct or an empty array, as
`#[derive(Clone)]` of a unit struct and `|_| [0u8; 0]` do, and a std adaptor of only
zero-sized fields, as `iter::from_fn` of a closure capturing nothing is.

## Plan

Found by `hints.rs` in the lang fixture. The first emits `return _0` of a return place
`unassigned` skips, which `tsc` rejects, and the second refuses as "constructing
`std::iter::FromFn<..>` from another crate". Once they translate, along with
[ts-unit-closures](/quest/m3/ts-unit-closures.md), `hints::reserves` covers a `Vec` of
zero-sized elements, which holds `usize::MAX`, and `hints::hints` a flatten of `[T; 0]`.

Public API: the translator's output.
