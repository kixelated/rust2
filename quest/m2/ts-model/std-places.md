# [M] Std's mutable references to values JS copies

## Goal

rust2-ts translates the std methods that hand out a `&mut` to a value JS copies (a
number, an `Option` of one, a `Box`), giving an `rt.Place` where
[#128](https://github.com/kixelated/rust2/pull/128)'s guard refuses them now, proven by
differential tests against native Rust.

## Plan

Found by #128 on 2026-10-27: its guard refused 74 std-made `&mut`s in moq-net, most from
`Option::as_mut`, `get_or_insert*`, `insert`, `slice::get_mut` and `last_mut`,
`iter_mut` and `values_mut`, `Entry::and_modify`, `retain_mut`, and `dedup_by_key`.
Also refused: an `Option<&mut T>` passed through `map`, `map_or`, `filter`, and
`and_then` closures, and comparing `Option`s or tuples of Places through the runtime's key
functions.

- Rerun the model's census first: [#129](https://github.com/kixelated/rust2/pull/129) now
  lowers `Option`'s combinators from core's MIR, so some of these may already translate.
- Code that needs no Place still gets none; measure on the
  [TS benchmark](/ts/README.md#bench).

Public API: the translator's output.
