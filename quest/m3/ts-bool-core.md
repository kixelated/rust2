# [XS] bool's combinators from core's MIR

## Goal

rust2-ts lowers `bool::then` and `then_some` from core's MIR, as
[#129](https://github.com/kixelated/rust2/pull/129) does `Option`'s and `Result`'s, and
deletes `rt.boolThen` and its dropper, with drops exact, proven by differential tests
against native Rust and the codec's bundle no larger.

## Plan

Found by [#115](https://github.com/kixelated/rust2/pull/115) on 2026-10-27: `rt.boolThen`
is the last runtime combinator that takes a closure dropper, and `rt.once` may go with it.
Decided with the maintainer on 2026-10-27 to plan it, since it deletes runtime code;
integer methods stay in the runtime, as core-mir decided.

Public API: the translator's output and the runtime.
