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
- [Strings from and into iterators](/quest/m3/ts-string-iters.md) - `collect::<String>`, `String::extend`, and `drain` translate
- [Other FromStr types](/quest/m3/ts-from-str.md) - `str::parse` of `bool`, `char`, and `NonZero` translates
- [Placeholders written to a crate's own fmt::Write](/quest/m3/ts-fmt-calls.md) - `write!` with placeholders into a crate's own writer translates, its `write_str` seeing each call std makes
- [Waiting: type-check in process with tsrs](/quest/m3/ts-tsrs.md) - rust2-ts checks bindings through tsrs instead of `tsc` once it is a stable library
- [An inclusive range folded through an adaptor](/quest/m3/ts-range-fold.md) - a `RangeInclusive` folded through `map`, `filter`, or `rev` ends in core's state, and `by_ref` of one type-checks
- [Binding gaps](/quest/m3/ts-bind-gaps.md) - bound types as keys, unit returns checked, renamed dependencies
- [A B-tree's drop order](/quest/m3/ts-btree-drop.md) - dropping a whole `BTreeMap` or `BTreeSet` drops its entries node by node, as std's does
- [Drop order in std's folds and iterators](/quest/m3/ts-drop-order.md) - the values that drop still refused in std's folds, zips, flattens, and collects drop as std's do
- [References to byte-array keys a collection yields](/quest/m3/ts-byte-key-refs.md) - `keys`, `iter`, `get_key_value`, and `HashSet::get` of a `[u8; N]` key translate
- [An array as a slice](/quest/m3/ts-array-as-slice.md) - `<[T; N]>::as_slice` and `as_mut_slice` translate
- [reduce, max_by, min_by, and Peekable's next_if_eq and peek_mut](/quest/m3/ts-iter-reducers.md) - the iterator consumers std returns an item from that have no mapping translate
- [Closures that return an Option whose None is rt.None](/quest/m3/ts-option-closures.md) - `find_map`, `filter_map`, and `map_while` to `Option<Option<u8>>` keep `None` apart from `Some(None)`
- [A slice's edits in place](/quest/m3/ts-slice-edits.md) - `fill`, `rotate_left`, and `copy_from_slice` of other values translate
- [A slice's parts by inclusive range, checked range, and fixed size](/quest/m3/ts-slice-ranges.md) - `&v[a..=b]`, `get_mut(a..b)`, and `first_chunk_mut` translate, writing through
- [Waiting: a slice object for a mutable part of other values](/quest/m3/ts-slice-object.md) - waits on a moq profile showing the `Proxy` view's traps matter, then decides on an `rt.ByteSlice`-like form
- [Port the workflow guard to kixelated/quest](/quest/m3/quest-workflow-guard.md) - quest's environment guard parses YAML as this repository's does
- [A parameter read only where lowering leaves nothing](/quest/m3/ts-unread-params.md) - an instance whose lowering drops every read of a parameter marks it unused, so `tsc` accepts it
