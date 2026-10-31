# [S] Fresh writes through std's iterators

## Goal

[rust2-ts](/ts/README.md) judges a whole write fresh when it reaches the memory through
std's slice and array iterators, as in
`for slot in &mut slots { slot.write(Cell::new(..)); }` over a `MaybeUninit` array or a
fresh `Box` of one, so a raw pointer into a slot's part taken after the loop translates.
A write that may run while a pointer into the memory exists stays refused with its span,
proven by differential tests against native Rust.

## Plan

Found while landing fresh writes: `Trace` in `ts/src/lower/mir/holders.rs` counts any
std call given a pointer to the crate's type as an escape, since std's code could run
the crate's on it, so `into_iter` and `IterMut::next` over a slice of the crate's slots
end the trace, and the loop's writes count as giving their parts other objects. Indexing
the same array, `slots[at].write(..)`, already translates.

- Std's iterators over references and pointers run none of the element's code: no trait
  bound on it and no drop of it. Prefer deciding that from the callee's bounds and what
  it may drop over a list of names, if it holds up.
- A consumer to name: moq writes slots by index today, so this waits on one that iterates.
