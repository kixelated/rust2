//! The entry point both sides of the differential test run. The first byte picks a
//! case and the rest is its input.

use crate::{fmt, ints, iters, shapes};

/// The native side's entry: [`run`], except that a case starting with 0xff prints a
/// [`fmt::Fault`], which the translated side prints from TypeScript in `caller.ts`.
pub fn entry(input: &[u8]) -> Vec<u8> {
	match input.split_first() {
		Some((0xff, data)) => fmt::printed(data),
		_ => run(input),
	}
}

/// Run one case.
pub fn run(input: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	let [op, kind, data @ ..] = input else {
		return out;
	};
	let (op, kind) = (*op, *kind);
	match op % 32 {
		0 => ints::u8_op(kind, data, &mut out),
		1 => ints::u16_op(kind, data, &mut out),
		2 => ints::u32_op(kind, data, &mut out),
		3 => ints::u64_op(kind, data, &mut out),
		4 => ints::u128_op(kind, data, &mut out),
		5 => ints::usize_op(kind, data, &mut out),
		6 => ints::i8_op(kind, data, &mut out),
		7 => ints::i16_op(kind, data, &mut out),
		8 => ints::i32_op(kind, data, &mut out),
		9 => ints::i64_op(kind, data, &mut out),
		10 => ints::i128_op(kind, data, &mut out),
		11 => ints::isize_op(kind, data, &mut out),
		12 => match kind % 6 {
			0 => ints::i8_neg(data, &mut out),
			1 => ints::i16_neg(data, &mut out),
			2 => ints::i32_neg(data, &mut out),
			3 => ints::i64_neg(data, &mut out),
			4 => ints::i128_neg(data, &mut out),
			_ => ints::isize_neg(data, &mut out),
		},
		13 => match data.split_first() {
			Some((&to, data)) => ints::cast(kind, to, data, &mut out),
			None => ints::cast(kind, kind / 12, data, &mut out),
		},
		14 => ints::convert(data, &mut out),
		15 => shapes::copies(data, &mut out),
		16 => shapes::loops(data, &mut out),
		17 => shapes::patterns(data, &mut out),
		18 => shapes::closures(data, &mut out),
		19 => shapes::discriminants(data, &mut out),
		20 => shapes::library(data, &mut out),
		21 => fmt::ints(kind, data, &mut out),
		22 => fmt::strings(data, &mut out),
		23 => fmt::derived(data, &mut out),
		24 => fmt::written(data, &mut out),
		25 => fmt::panics(kind, data, &mut out),
		26 => iters::adaptors(kind, data, &mut out),
		27 => iters::consumers(kind, data, &mut out),
		28 => iters::ranges(data, &mut out),
		29 => iters::options(data, &mut out),
		30 => iters::own(data, &mut out),
		_ => iters::values(data, &mut out),
	}
	out
}
