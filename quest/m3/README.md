# m3: deferred

## Goal

Work that waits on evidence or has no consumer yet: the build-speed tracks,
each a measurement that decides whether its follow-up lands, and the parts of
std the translators cover before a consumer calls them.

## Plan

Decided with the maintainer on 2026-10-27: rust2 aims to support all of std, so a
generic std gap is planned here even when moq never calls it.

## Required

- [Retrial Cranelift on moq's tests](/quest/m3/cranelift-unwind.md) - waits on upstream: moq's tests pass under Cranelift, so its CI can weigh it again
- [Relink without rebuilding](/quest/m3/relink.md) - measure upstream's skip of dependency-only rebuilds on moq's edits once it reaches nightly
- [Fresh writes through std's iterators](/quest/m3/ts-fresh-iterated.md) - a whole write reaching fresh memory through std's slice iterators no longer refuses later pointers, as writing by index already doesn't
- [Unsafe code the translator can model](/quest/m3/ts-unsafe.md) - a dependency using only `unsafe` the translator can model translates from source
- [The capacity of a BytesMut](/quest/m3/ts-bytes-capacity.md) - `BytesMut::capacity` and what reads it translate with `bytes`' values
- [BufMut's provided writes through a &mut](/quest/m3/ts-buf-mut-forward.md) - `put`, `put_bytes`, `put_uint`, and `put_int` of a `&mut Vec<u8>` or `&mut BytesMut` translate
- [Trait objects](/quest/m3/ts-dyn.md) - `dyn Error`, `dyn Iterator`, `dyn Buf`, and `Arc<dyn Debug>` translate, so a consumer neither rewrites nor binds them
- [Placeholders written to a crate's own fmt::Write](/quest/m3/ts-fmt-calls.md) - `write!` with placeholders into a crate's own writer translates, its `write_str` seeing each call std makes
- [Waiting: type-check in process with tsrs](/quest/m3/ts-tsrs.md) - rust2-ts checks bindings through tsrs instead of `tsc` once it is a stable library
- [Drop order in std's folds and iterators](/quest/m3/ts-drop-order.md) - the values that drop still refused in std's consumers, flattens, and collects drop as std's do
- [References to byte arrays std is given](/quest/m3/ts-byte-refs-given.md) - `Option<&[u8; N]>::copied`, `mem::swap` through a `&mut [u8; N]`, and std's other functions given a reference to an array they take as `&T` translate
- [Waiting: a slice object for a mutable part of other values](/quest/m3/ts-slice-object.md) - waits on a moq profile showing the `Proxy` view's traps matter, then decides on an `rt.ByteSlice`-like form
- [Port the workflow guard to kixelated/quest](/quest/m3/quest-workflow-guard.md) - quest's environment guard parses YAML as this repository's does
- [A deque's two slices](/quest/m3/ts-deque-slices.md) - `VecDeque::as_slices` and `{:?}` of a deque's `Iter`, `IterMut`, and `Drain` translate, split where std's are
- [mem::replace and mem::swap of ()](/quest/m3/ts-unit-replace.md) - swapping or replacing through a `&mut ()` type-checks and matches native Rust
- [LinkedList](/quest/m3/ts-linked-list.md) - `LinkedList` and its methods translate
- [Saturating](/quest/m3/ts-saturating.md) - `Saturating<T>` and its std impls translate
- [Zip's next_back in constant time](/quest/m3/ts-zip-back.md) - a reversed zip of random-access iterators skips the longer side's excess at once
- [A B-tree's collect by std's sort and dedup](/quest/m3/ts-btree-collect-order.md) - collecting into a B-tree keeps the keys std's does when a key's impls disagree
- [A tuple's cmp typed as total](/quest/m3/ts-cmp-tuple-total.md) - `rt.cmpTuple` of total fields types its result as `Ordering`
- [concat and join of slices of arrays](/quest/m3/ts-slice-concat.md) - `concat` and `join` of a slice of arrays or slices translate
