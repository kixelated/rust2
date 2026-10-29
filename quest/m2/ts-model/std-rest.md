# [M] The rest of std the model reaches

## Goal

rust2-ts translates the std calls the model's census still refuses outside other quests:
`Vec`'s and `VecDeque`'s `capacity`, `reserve`, and `shrink_to`, and `RandomState` with
`BuildHasher::hash_one`, also where another crate names it, as hashbrown's and hashlink's
default hasher does, proven by differential tests against native Rust.

## Plan

Left by [#168](https://github.com/kixelated/rust2/pull/168) on 2026-10-28, which translated
the rest of what the census refused in std. Decided with the maintainer on 2026-10-28:

- **Capacity: std's exact growth, everywhere.** moq-net reads `capacity` to decide when to
  prune (`cache.rs`'s `hold_head`, `resume.rs`'s readers) and bills `VecDeque` growth by
  it (`group.rs`), so a capacity other than std's changes what it does. Every `Vec` and
  `VecDeque` carries std's capacity (`RawVec::grow_amortized`, its minimum non-zero
  capacity by element size, `with_capacity` exact, `shrink_to`, and what `collect` and
  `clone` allocate), through every runtime function that allocates. Measure push cost on
  the TS benchmark before and after.
- **`RandomState`: random per state, from `crypto.getRandomValues`**, as std seeds from
  the OS. It keys a treap's priorities in `track/demand/ranges.rs`, so a differential test
  compares only what the values decide, such as equal inputs hashing equally.

Public API: the translator's output.
