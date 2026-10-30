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
- [A raw pointer's staleness bound by its liveness](/quest/m3/ts-pointer-liveness.md) - a pointer dead before its variable is assigned again, as in an earlier loop iteration, translates
- [Writes into fresh memory before any pointer to it](/quest/m3/ts-fresh-writes.md) - a whole write filling a fresh allocation or `MaybeUninit` before any pointer into it no longer refuses later pointers, as kio's `Segment::new` slots
- [Unsafe code the translator can model](/quest/m3/ts-unsafe.md) - a dependency using only `unsafe` the translator can model translates from source
- [The capacity of a BytesMut](/quest/m3/ts-bytes-capacity.md) - `BytesMut::capacity` and what reads it translate with `bytes`' values
- [BufMut's provided writes through a &mut](/quest/m3/ts-buf-mut-forward.md) - `put`, `put_bytes`, `put_uint`, and `put_int` of a `&mut Vec<u8>` or `&mut BytesMut` translate
- [Trait objects](/quest/m3/ts-dyn.md) - `dyn Error`, `dyn Iterator`, `dyn Buf`, and `Arc<dyn Debug>` translate, so a consumer neither rewrites nor binds them
- [Placeholders written to a crate's own fmt::Write](/quest/m3/ts-fmt-calls.md) - `write!` with placeholders into a crate's own writer translates, its `write_str` seeing each call std makes
- [Waiting: type-check in process with tsrs](/quest/m3/ts-tsrs.md) - rust2-ts checks bindings through tsrs instead of `tsc` once it is a stable library
- [An inclusive range folded through an adaptor](/quest/m3/ts-range-fold.md) - a `RangeInclusive` folded through `map`, `filter`, or `rev` ends in core's state, and `by_ref` of one type-checks
- [Binding gaps](/quest/m3/ts-bind-gaps.md) - bound types as keys, unit returns checked, renamed dependencies
- [A B-tree's drop order](/quest/m3/ts-btree-drop.md) - dropping a whole `BTreeMap` or `BTreeSet` drops its entries node by node, as std's does
- [Drop order in std's folds and iterators](/quest/m3/ts-drop-order.md) - the values that drop still refused in std's folds, zips, flattens, and collects drop as std's do
- [Set operations](/quest/m3/ts-set-ops.md) - `union`, `intersection`, `difference`, and `symmetric_difference` of hash and B-tree sets translate
- [References to byte arrays std takes](/quest/m3/ts-byte-refs-taken.md) - `extend` by references to arrays, adaptors' closures over held arrays, and a collection of both forms translate
- [Peekable::next_if_eq against another type](/quest/m3/ts-next-if-eq-other.md) - `next_if_eq` with an expected value of another type than the item, such as a `&str` of `String` items, translates
- [A closure's () result as a value](/quest/m3/ts-unit-closures.md) - a closure returning `()` whose result is a value, as in a `Vec<()>` collected from `map`, type-checks
- [std accessors with no mapping](/quest/m3/ts-unmapped-accessors.md) - `VecDeque::swap_remove_*`, `HashSet::replace`, `BTreeSet::get`, `take`, and `replace`, `Vec::pop_if`, and `Arc` and `Rc::into_inner` translate
- [Integers' little- and native-endian bytes](/quest/m3/ts-int-ne-bytes.md) - each integer's `from_le_bytes`, `to_ne_bytes`, and `from_ne_bytes` translate
- [mul_add rounded once](/quest/m3/ts-mul-add.md) - `f64::mul_add` and `f32::mul_add` translate, rounding once as std's fused multiply-add does
- [Waiting: a slice object for a mutable part of other values](/quest/m3/ts-slice-object.md) - waits on a moq profile showing the `Proxy` view's traps matter, then decides on an `rt.ByteSlice`-like form
- [Port the workflow guard to kixelated/quest](/quest/m3/quest-workflow-guard.md) - quest's environment guard parses YAML as this repository's does
- [A parameter read only where lowering leaves nothing](/quest/m3/ts-unread-params.md) - an instance whose lowering drops every read of a parameter marks it unused, so `tsc` accepts it
- [Zero-sized values made out of sight](/quest/m3/ts-zero-sized.md) - unit structs and empty arrays returned, and std adaptors of only zero-sized fields, translate
- [Array keys a collection holds](/quest/m3/ts-array-keys.md) - collections of `[T; N]` keys type-check
- [Zip's next_back in constant time](/quest/m3/ts-zip-back.md) - a reversed zip of random-access iterators skips the longer side's excess at once
