# [M] Writes through places the model still refuses

## Goal

rust2-ts translates the writes through a place that the model's census still refuses,
proven by differential tests against native Rust:

- `Option::get_or_insert_default` on an `Option` reached through a `&mut`, such as a
  `&mut Option<Vec<_>>` from a match on `&mut self.fields` (moq-net's `group.rs`), and the
  call it is still refused as outright (`origin.rs`).
- `mem::take` of a `&mut Vec` a closure captured (`origin.rs`), refused as "replacing the
  value behind a reference".

## Plan

Found by [#168](https://github.com/kixelated/rust2/pull/168) on 2026-10-28: four
`get_or_insert_default` refusals stayed with [#149](https://github.com/kixelated/rust2/pull/149)'s
`rt.Place`s landed, and no quest owns them since #149 deleted its own.

- Both are a place the lowering can't name: a captured `&mut` is a capture of a place, not a
  std gap, and `get_or_insert_default` already lowers in `call.rs` where its place guard
  allows. Find what each guard refuses, and widen it only as far as the semantics stay
  exact.
- Report the model's census before and after.

Public API: the translator's output.
