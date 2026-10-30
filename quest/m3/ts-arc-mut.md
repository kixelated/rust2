# [S] Arc's unique-access functions

## Goal

`Arc::get_mut`, `Arc::make_mut`, `Arc::pin`, and `Option::as_deref` of an `Arc` translate,
proven by differential tests against native Rust, including the strong count `make_mut`
leaves when it clones.

## Plan

Found by [#220](https://github.com/moq-dev/rust2/pull/220) on 2026-10-29, where each is
refused as an unmapped call. A `&mut Arc` is an `rt.Place` since #220, so `make_mut`'s
clone goes into the place's `v` when the count is above one, and `get_mut` gives the
value only when the strong count is one and no `Weak` is left.
