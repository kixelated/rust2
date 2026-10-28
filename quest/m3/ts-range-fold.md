# [S] An inclusive range folded through an adaptor

## Goal

A `RangeInclusive` borrowed by `by_ref` and drained by a fold through an adaptor (`map`,
`filter`, `rev`, `chain`, `peekable`, `take`, `skip`, ...) ends in core's state, `exhausted`
with `start == end`, where std's adaptor folds the range, proven by a differential test
against native Rust.

## Plan

[#179](https://github.com/kixelated/rust2/pull/179) gave `rt.Iter` a `foldNext`, which each
consumer std folds reads by, and which `RangeInclusive` and a `&mut` borrow take as core's
`try_fold` does. What remains is each adaptor:

- One whose std `try_fold` folds its iterator overrides `foldNext` to read its iterator's.
- `filter`, `filter_map`, and `skip_while` fold it in `next` too, through `find`, as the
  `collect` shunts do through `try_for_each`.
- `rev` needs a back step, `foldNextBack`, as core's `try_rfold` takes it.
- `skip` reads `nth(n - 1)` before it folds, where its `next` reads `nth(n)`.
- `zip` and `step_by` read by `next`, as std's do.
- `collect` and `extend` fold only a `TrustedLen` iterator and read others by `next`.

Public API: the translator's output.
