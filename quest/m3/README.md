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
- [Unsafe code the translator can model](/quest/m3/ts-unsafe.md) - a dependency using only `unsafe` the translator can model translates from source
- [f32](/quest/m3/ts-f32.md) - `f32` and `Duration`'s `f32` methods translate as `f64` does
- [A mutable reference to a reference](/quest/m3/ts-mut-ref.md) - `&mut &[u8]` and `&mut &mut T` translate, so `Buf` of a `&[u8]` and `bytes`' forwarding impls do
- [The capacity of a BytesMut](/quest/m3/ts-bytes-capacity.md) - `BytesMut::capacity` and what reads it translate with `bytes`' values
- [BufMut's provided writes through a &mut](/quest/m3/ts-buf-mut-forward.md) - `put`, `put_bytes`, `put_uint`, and `put_int` of a `&mut Vec<u8>` or `&mut BytesMut` translate
- [Trait objects](/quest/m3/ts-dyn.md) - `dyn Error`, `dyn Iterator`, and `dyn Buf` translate, so a consumer neither rewrites nor binds them
- [fmt::Write](/quest/m3/ts-fmt-write.md) - `write!` into a `String`, through a `&mut`, and a crate's own `fmt::Write` impl translate
- [Waiting: type-check in process with tsrs](/quest/m3/ts-tsrs.md) - rust2-ts checks bindings through tsrs instead of `tsc` once it is a stable library
- [An inclusive range folded through an adaptor](/quest/m3/ts-range-fold.md) - a `RangeInclusive` folded through `map`, `filter`, or `rev` ends in core's state, and `by_ref` of one type-checks
- [Binding gaps](/quest/m3/ts-bind-gaps.md) - bound types as keys, unit returns checked, renamed dependencies
- [Drop order in std's folds and iterators](/quest/m3/ts-drop-order.md) - the values that drop still refused in std's folds, zips, flattens, and collects drop as std's do
- [Checks on the differential harnesses](/quest/m3/ts-harness.md) - step lists that drift, literal op bytes, and `refs`'s coverage fail a test or get cases
- [Ordering in std](/quest/m3/ts-ordering.md) - `cmp::max`/`min`, `is_sorted`, and `BinaryHeap`, and #173's cost on integers measured
- [Arc's unique-access functions](/quest/m3/ts-arc-mut.md) - `Arc::get_mut`, `make_mut`, `pin`, and `as_deref` of an `Option<Arc>` translate
- [Debug and Display of Arc and ranges](/quest/m3/ts-fmt-wrappers.md) - `Arc<T>` formats as `T`, and ranges as std's `Debug`
- [Fold a match a bool decided](/quest/m3/ts-fold-bool.md) - `if matches!` and derived `==` drop a later match's dead arms
- [Rename one instance](/quest/m3/ffi-instance-rename.md) - `rename` by an instance's path, once a consumer's clash needs it
- [Into targets through a chain of From implementations](/quest/m3/ffi-from-chains.md) - `sources` follows more than one level, once a consumer's parameter needs it
- [Port the workflow guard to kixelated/quest](/quest/m3/quest-workflow-guard.md) - quest's environment guard parses YAML as this repository's does
