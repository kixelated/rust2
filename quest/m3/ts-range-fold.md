# [S] An inclusive range folded through an adaptor

## Goal

A `RangeInclusive` borrowed by `by_ref` and drained by a fold through an adaptor (`map`,
`filter`, `rev`, `chain`, `peekable`, `take`, `skip`, ...) ends in core's state, `exhausted`
with `start == end`, where std's adaptor folds the range, proven by a differential test
against native Rust.

## Plan

[#179](https://github.com/kixelated/rust2/pull/179) gave `rt.Iter` a `folded()`, which each
consumer std folds reads through: the iterator itself, but a stepper of core's `try_fold`
for a `RangeInclusive`, and its iterator's for a `&mut` borrow. A per-item method instead
cost up to 58% on `just ts bench`. [#172](https://github.com/kixelated/rust2/pull/172)'s
per-kind `fold`, which consumers take where the iterator drops, forwards through each
adaptor to the default `fold`, which reads through `folded()`. What remains is each
adaptor where a consumer reads it by `next`:

- One whose std `try_fold` folds its iterator overrides `folded()` to read its iterator's
  `folded()`, sharing its own state (`take`'s count, `peekable`'s peeked item) with it.
- `filter`, `filter_map`, and `skip_while` fold it in `next` too, through `find`, as the
  `collect` shunts do through `try_for_each`.
- `rev` needs a back step, as core's `try_rfold` takes it.
- `skip` reads `nth(n - 1)` before it folds, where its `next` reads `nth(n)`.
- `zip` and `step_by` read by `next`, as std's do.
- `collect` and `extend` fold only a `TrustedLen` iterator and read others by `next`.

Public API: the translator's output.
