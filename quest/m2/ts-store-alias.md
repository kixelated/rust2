# [S] Copy what a container stores

## Goal

Storing a by-value `Copy` aggregate in a container copies it, as Rust does:
`v.push(pair); v.push(pair); v[0].0 ^= 1` leaves `v[1]` and `pair` unchanged in
translated TypeScript. Differential tests against native Rust cover `Vec::push`,
`insert`, `VecDeque::push_back`, map inserts, and `Box::new`.

## Plan

Found by [#86](https://github.com/kixelated/rust2/pull/86) on 2026-10-26, and it
still reproduces after [#83](https://github.com/kixelated/rust2/pull/83)'s typed
copies: 1280 differences against native Rust in its probe. It is a silent miscompile.

- The cause is the share analysis in `ts/src/lower/mir/mod.rs`: it sees a later change
  made in place through a local, not through a container element, so it shares the
  argument instead of copying it.
- Fix it there, once for every std call that stores a by-value argument, not in each
  mapping.
- Measure the codec bench before and after.

Public API: the translator's output.
