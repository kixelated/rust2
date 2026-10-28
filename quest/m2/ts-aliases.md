# [M] Held values that name a reassigned variable

## Goal

Wherever rust2-ts holds an expression back instead of reading it once, the held value
reads what Rust read when it was made, even after a variable it names is assigned again,
including on a later iteration of a loop. Proven by differential tests against native Rust
that reassign each name between the hold and the read, in and across loop iterations.

## Plan

Collected on 2026-10-28 from three reports of one shape:

- `stable_path` holds only the outermost object of a place, so an index inside it
  (`a[i].b[j]` with `i` reassigned while the borrow lives) is read live
  ([#176](https://github.com/kixelated/rust2/pull/176)).
- `alias()`'s copy arm treats a held-back source local as stable, though its folded value
  may name a variable assigned again ([#148](https://github.com/kixelated/rust2/pull/148)).
  No miscompile was found through it, but it is #148's bug in another place.
- Values made in a loop that share one JS binding across iterations where each Rust
  iteration has its own value.

Audit every place the lowering folds or holds an expression rather than fixing these one
by one: the rule is one check of which names a held value reads and whether any can be
written before the read. #148's review also asked for differential coverage of reborrows
through a reassigned reference and of indexed borrows.

Public API: the translator's output.
