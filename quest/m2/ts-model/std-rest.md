# [M] The rest of std the model reaches

## Goal

rust2-ts translates the std calls the model's census still refuses outside other quests:
`Vec`'s and `VecDeque`'s `capacity`, `reserve`, and `shrink_to`, and `RandomState` with
`BuildHasher::hash_one`, proven by differential tests against native Rust.

## Plan

Left by [#168](https://github.com/kixelated/rust2/pull/168) on 2026-10-28, which translated
the rest of what the census refused in std:

- moq-net reads `capacity` to decide when to prune (`cache.rs`'s `hold_head`,
  `resume.rs`'s readers) and bills `VecDeque` growth by it (`group.rs`), so a capacity
  other than std's changes what it does. std's growth (`RawVec::grow_amortized`, its
  minimum non-zero capacity by element size, `with_capacity` exact, `shrink_to`) is
  deterministic on the pinned toolchain. Open: whether every `Vec` and `VecDeque` carries
  that capacity through every runtime function that allocates, or the translator tracks it
  only where a program reads it.
- `RandomState` keys a treap's priorities in `track/demand/ranges.rs`. Its values are
  random in std, so a differential test can compare only what they decide. Open: whether
  the runtime seeds it from `crypto.getRandomValues`, as std seeds from the OS.

Public API: the translator's output.
