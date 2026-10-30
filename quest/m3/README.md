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
- [The capacity of a BytesMut](/quest/m3/ts-bytes-capacity.md) - `BytesMut::capacity` and what reads it translate with `bytes`' values
- [BufMut's provided writes through a &mut](/quest/m3/ts-buf-mut-forward.md) - `put`, `put_bytes`, `put_uint`, and `put_int` of a `&mut Vec<u8>` or `&mut BytesMut` translate
- [Trait objects](/quest/m3/ts-dyn.md) - `dyn Error`, `dyn Iterator`, `dyn Buf`, and `Arc<dyn Debug>` translate, so a consumer neither rewrites nor binds them
- [fmt::Write](/quest/m3/ts-fmt-write.md) - `write!` into a `String`, through a `&mut`, and a crate's own `fmt::Write` impl translate
- [String's edits](/quest/m3/ts-string-edits.md) - `push_str`, `clear`, and `String`'s other edits in place translate
- [Waiting: type-check in process with tsrs](/quest/m3/ts-tsrs.md) - rust2-ts checks bindings through tsrs instead of `tsc` once it is a stable library
- [An inclusive range folded through an adaptor](/quest/m3/ts-range-fold.md) - a `RangeInclusive` folded through `map`, `filter`, or `rev` ends in core's state, and `by_ref` of one type-checks
- [Binding gaps](/quest/m3/ts-bind-gaps.md) - bound types as keys, unit returns checked, renamed dependencies
- [Drop order in std's folds and iterators](/quest/m3/ts-drop-order.md) - the values that drop still refused in std's folds, zips, flattens, and collects drop as std's do
- [Byte arrays compared for equality and hashed](/quest/m3/ts-byte-keys.md) - `==` of byte arrays, and byte arrays, slices, and vectors as hash keys, translate
- [Ranges of Bounds](/quest/m3/ts-bound-ranges.md) - drains and B-tree ranges of a `(Bound, Bound)` translate
- [The rest of a slice's chunks](/quest/m3/ts-slice-chunks.md) - `chunks_exact`, `rchunks`, and the mutable chunks translate
- [Iterator consumers over items whose None is rt.None](/quest/m3/ts-option-items.md) - `nth`, `last`, and `find` of an iterator of `Option` keep `None` apart from `Some(None)`
- [Port the workflow guard to kixelated/quest](/quest/m3/quest-workflow-guard.md) - quest's environment guard parses YAML as this repository's does
