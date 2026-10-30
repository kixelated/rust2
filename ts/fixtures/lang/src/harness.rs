//! The entry point both sides of the differential test run. The first byte picks a
//! case and the rest is its input.

use crate::{
	arrays, atomics, borrows, bounds, captures, cells, chars, chunks, collections, combinators, counts, cows, defaults,
	durations, dyns, edits, errors, floats, fmt, generics, hashes, holds, ints, iters, keys, locals, locks, nested,
	objects, orders, pointers, refs, shapes, slices, splits, statics, strs, utf8, wrappers,
};

/// The native side's entry: [`run`], except that a case starting with 0xff prints a
/// [`fmt::Fault`], which the translated side prints from TypeScript in `caller.ts`.
pub fn entry(input: &[u8]) -> Vec<u8> {
	match input.split_first() {
		Some((0xff, data)) => fmt::printed(data),
		_ => run(input),
	}
}

/// What a case runs on the second byte and the rest.
type Case = fn(u8, &[u8], &mut Vec<u8>);

/// Each case's name and what it runs. The first byte picks one modulo their count, so a
/// case added at the end moves no other.
fn cases() -> Vec<(&'static str, Case)> {
	vec![
		("ints::u8_op", ints::u8_op),
		("ints::u16_op", ints::u16_op),
		("ints::u32_op", ints::u32_op),
		("ints::u64_op", ints::u64_op),
		("ints::u128_op", ints::u128_op),
		("ints::usize_op", ints::usize_op),
		("ints::i8_op", ints::i8_op),
		("ints::i16_op", ints::i16_op),
		("ints::i32_op", ints::i32_op),
		("ints::i64_op", ints::i64_op),
		("ints::i128_op", ints::i128_op),
		("ints::isize_op", ints::isize_op),
		("ints::neg", neg),
		("ints::cast", cast),
		("ints::convert", |_, data, out| ints::convert(data, out)),
		("shapes::copies", |_, data, out| shapes::copies(data, out)),
		("shapes::loops", |_, data, out| shapes::loops(data, out)),
		("shapes::patterns", |_, data, out| shapes::patterns(data, out)),
		("shapes::closures", |_, data, out| shapes::closures(data, out)),
		("shapes::discriminants", |_, data, out| shapes::discriminants(data, out)),
		("shapes::library", |_, data, out| shapes::library(data, out)),
		("fmt::ints", fmt::ints),
		("fmt::strings", |_, data, out| fmt::strings(data, out)),
		("fmt::derived", |_, data, out| fmt::derived(data, out)),
		("fmt::written", |_, data, out| fmt::written(data, out)),
		("fmt::panics", fmt::panics),
		("generics::methods", |_, data, out| generics::methods(data, out)),
		("iters::adaptors", iters::adaptors),
		("iters::consumers", iters::consumers),
		("iters::ranges", |_, data, out| iters::ranges(data, out)),
		("iters::options", |_, data, out| iters::options(data, out)),
		("iters::own", |_, data, out| iters::own(data, out)),
		("iters::values", |_, data, out| iters::values(data, out)),
		("shapes::order", |_, data, out| shapes::order(data, out)),
		("collections::hash_maps", collections::hash_maps),
		("collections::sets", collections::sets),
		("collections::btree_maps", collections::btree_maps),
		("collections::deques", |_, data, out| collections::deques(data, out)),
		("shapes::flows", |_, data, out| shapes::flows(data, out)),
		("keys::keys", keys::keys),
		("chars::chars", chars::chars),
		("strs::strs", strs::strs),
		("collections::vecs", |_, data, out| collections::vecs(data, out)),
		("strs::slices", strs::slices),
		("collections::stores", |_, data, out| collections::stores(data, out)),
		("chunks::chunks", chunks::chunks),
		("cows::cows", cows::cows),
		("durations::durations", durations::durations),
		("combinators::combinators", combinators::combinators),
		("splits::splits", splits::splits),
		("nested::nested", nested::nested),
		("refs::refs", |_, data, out| refs::refs(data, out)),
		("fmt::counts", |_, data, out| fmt::counts(data, out)),
		("fmt::rounding", |_, _, out| fmt::rounding(out)),
		("iters::skips", iters::skips),
		("wrappers::wrappers", wrappers::wrappers),
		("edits::edits", edits::edits),
		("utf8::decodes", utf8::decodes),
		("floats::floats", floats::floats),
		("atomics::atomics", atomics::atomics),
		("keys::overrides", keys::overrides),
		("dyns::dyns", |_, data, out| dyns::dyns(data, out)),
		("cells::cells", cells::cells),
		("dyns::shared", |_, data, out| dyns::shared(data, out)),
		("iters::ranked", iters::ranked),
		("captures::captures", captures::captures),
		("locks::locks", |_, data, out| locks::locks(data, out)),
		("locks::relock", |_, data, out| locks::relock(data, out)),
		("locals::locals", |_, data, out| locals::locals(data, out)),
		("statics::statics", |_, data, out| statics::statics(data, out)),
		("locks::rwlocks", |_, data, out| locks::rwlocks(data, out)),
		("borrows::borrows", |_, data, out| borrows::borrows(data, out)),
		("defaults::defaults", |_, data, out| defaults::defaults(data, out)),
		("counts::arcs", |_, data, out| counts::arcs(data, out)),
		("counts::rcs", |_, data, out| counts::rcs(data, out)),
		("counts::cyclic_arcs", |_, data, out| counts::cyclic_arcs(data, out)),
		("counts::cyclic_rcs", |_, data, out| counts::cyclic_rcs(data, out)),
		("counts::unique_arcs", |_, data, out| counts::unique_arcs(data, out)),
		("counts::unique_rcs", |_, data, out| counts::unique_rcs(data, out)),
		("dyns::anys", |_, data, out| dyns::anys(data, out)),
		("dyns::fns", |_, data, out| dyns::fns(data, out)),
		("pointers::copies", |_, data, out| pointers::copies(data, out)),
		("pointers::lists", |_, data, out| pointers::lists(data, out)),
		("pointers::nested", |_, data, out| pointers::nested(data, out)),
		("pointers::places", |_, data, out| pointers::places(data, out)),
		("pointers::sentinels", |_, data, out| pointers::sentinels(data, out)),
		("pointers::shared", |_, data, out| pointers::shared(data, out)),
		("pointers::unchecked", |_, data, out| pointers::unchecked(data, out)),
		("edits::bytes", edits::bytes),
		("arrays::from_fns", |_, data, out| arrays::from_fns(data, out)),
		("collections::optionals", collections::optionals),
		("objects::objects", |_, data, out| objects::objects(data, out)),
		("holds::holds", holds::holds),
		("fmt::wrapped", |_, data, out| fmt::wrapped(data, out)),
		("refs::to_refs", |_, data, out| refs::to_refs(data, out)),
		("orders::orders", orders::orders),
		("counts::cloned_arcs", |_, data, out| counts::cloned_arcs(data, out)),
		("counts::cloned_rcs", |_, data, out| counts::cloned_rcs(data, out)),
		("counts::copies", |_, data, out| counts::copies(data, out)),
		("statics::cells", |_, data, out| statics::cells(data, out)),
		("iters::returned", |_, data, out| iters::returned(data, out)),
		("hashes::values", hashes::values),
		("hashes::random", |_, data, out| hashes::random(data, out)),
		("floats::formats", floats::formats),
		("floats::parses", floats::parses),
		("orders::bytes", |_, data, out| orders::bytes(data, out)),
		("slices::parts", slices::parts),
		("slices::drains", slices::drains),
		("slices::chunked", slices::chunked),
		("statics::onces", |_, data, out| statics::onces(data, out)),
		("fmt::writes", |_, data, out| fmt::writes(data, out)),
		("bounds::bounds", bounds::bounds),
		("pointers::cells", |_, data, out| pointers::cells(data, out)),
		("ints::parses", ints::parses),
		("strs::edits", strs::edits),
		("slices::subslices", slices::subslices),
		("hashes::bytes", |_, data, out| hashes::bytes(data, out)),
		("keys::bytes", keys::bytes),
		("strs::parses", strs::parses),
		("pointers::rebound", |_, data, out| pointers::rebound(data, out)),
		("slices::ranges", slices::ranges),
		("iters::reducers", |_, data, out| iters::reducers(data, out)),
		("pointers::writers", |_, data, out| pointers::writers(data, out)),
		("errors::eqs", errors::eqs),
		("slices::edits", slices::edits),
		("strs::iters", strs::iters),
		("strs::drains", strs::drains),
		("strs::hinted", strs::hinted),
		("arrays::as_slices", |_, data, out| arrays::as_slices(data, out)),
	]
}

/// Run one case.
pub fn run(input: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	let [op, kind, data @ ..] = input else {
		return out;
	};
	let cases = cases();
	let (_, case) = cases[usize::from(*op) % cases.len()];
	case(*kind, data, &mut out);
	out
}

/// The first byte that picks the case named `name`, which `caller.ts` targets by.
pub fn op(name: &str) -> u8 {
	match cases().iter().position(|&(n, _)| n == name) {
		Some(i) => i as u8,
		None => panic!("no case {name}"),
	}
}

/// Negate the signed type `kind` picks.
fn neg(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 6 {
		0 => ints::i8_neg(data, out),
		1 => ints::i16_neg(data, out),
		2 => ints::i32_neg(data, out),
		3 => ints::i64_neg(data, out),
		4 => ints::i128_neg(data, out),
		_ => ints::isize_neg(data, out),
	}
}

/// Cast from the type `kind` picks to the one the next byte does, or without one, to
/// a type `kind` also picks.
fn cast(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match data.split_first() {
		Some((&to, data)) => ints::cast(kind, to, data, out),
		None => ints::cast(kind, kind / 12, data, out),
	}
}
