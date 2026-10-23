//! The entry point both sides of the differential test run. The first byte picks a
//! case and the rest is its input.

use crate::{ints, shapes};

/// Run one case.
pub fn run(input: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	let [op, kind, data @ ..] = input else {
		return out;
	};
	let (op, kind) = (*op, *kind);
	match op % 23 {
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
		_ => shapes::library(data, &mut out),
	}
	out
}
