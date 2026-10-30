//! Iterators: std's adaptors and consumers, ranges, iterators of `Option`, and the
//! crate's own `impl Iterator`. Closures with effects log each call, so laziness and
//! where a consumer stops are part of the output.

use std::cmp::{Ordering, Reverse};
use std::fmt;

/// Adaptor chains over the input bytes, each consumed one way.
pub fn adaptors(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 8 {
		0 => {
			let doubled: Vec<u8> = data.iter().map(|&b| b / 2).filter(|b| b % 3 != 0).collect();
			out.extend_from_slice(&doubled);
			let small: Vec<u16> = data
				.iter()
				.filter_map(|&b| (b < 100).then_some(b as u16 * 200))
				.collect();
			for v in small {
				out.extend_from_slice(&v.to_be_bytes());
			}
		}
		1 => {
			for (i, &b) in data.iter().enumerate().skip(1).step_by(2) {
				out.push(i as u8);
				out.push(b);
			}
			for (i, b) in data.iter().rev().enumerate().take(3) {
				out.push(i as u8 ^ b);
			}
		}
		2 => {
			let (front, back) = data.split_at(data.len() / 2);
			let zipped: Vec<u8> = front.iter().zip(back.iter().rev()).map(|(a, b)| a ^ b).collect();
			out.extend_from_slice(&zipped);
			let chained: Vec<u8> = front.iter().chain(back).copied().rev().collect();
			out.extend_from_slice(&chained);
		}
		3 => {
			let taken: Vec<u8> = data.iter().copied().take_while(|&b| b != 0).collect();
			let skipped: Vec<u8> = data.iter().copied().skip_while(|&b| b < 0x80).collect();
			let mapped: Vec<u8> = data.iter().map_while(|&b| b.checked_add(0x40)).collect();
			out.push(taken.len() as u8);
			out.push(skipped.len() as u8);
			out.extend_from_slice(&mapped);
		}
		4 => {
			let words: Vec<Vec<u8>> = data.chunks_by_zero();
			let flat: Vec<u8> = words.iter().flat_map(|w| w.iter().rev()).copied().collect();
			out.extend_from_slice(&flat);
			let nested = words.iter().map(|w| w.iter().map(|&b| b.wrapping_add(3))).flatten();
			out.extend(nested.rev());
		}
		5 => {
			let mut it = data.iter().peekable();
			while let Some(&b) = it.next() {
				let mut run = 1u8;
				while it.next_if(|&&next| next == b).is_some() {
					run += 1;
				}
				out.push(b);
				out.push(run);
				if let Some(&&next) = it.peek() {
					out.push(next);
				}
			}
		}
		6 => {
			let mut it = data.iter();
			let firsts: Vec<u8> = it.by_ref().take(2).copied().collect();
			out.extend_from_slice(&firsts);
			out.push(it.len() as u8);
			match it.next_back() {
				Some(&last) => out.push(last),
				None => out.push(0xee),
			}
			let rest: Vec<u8> = it.rev().step_by(3).copied().collect();
			out.extend_from_slice(&rest);
		}
		_ => {
			// Each element of `pairs` is a copy: changing one changes nothing else.
			let pairs: Vec<[u8; 2]> = data.iter().map(|&b| [b, !b]).collect();
			let mut copies: Vec<[u8; 2]> = pairs.iter().copied().collect();
			if !copies.is_empty() {
				copies[0][0] = 0x55;
			}
			for pair in pairs.iter().chain(copies.iter()) {
				out.extend_from_slice(pair);
			}
			let mut v: Vec<u16> = Vec::new();
			v.extend(data.iter().map(|&b| b as u16 + 1));
			out.push(v.len() as u8);
			out.extend(data.iter().filter(|&&b| b > 0x80));
		}
	}
}

trait ChunksByZero {
	fn chunks_by_zero(&self) -> Vec<Vec<u8>>;
}

impl ChunksByZero for [u8] {
	/// The runs between zero bytes.
	fn chunks_by_zero(&self) -> Vec<Vec<u8>> {
		let mut words = Vec::new();
		let mut word = Vec::new();
		for &b in self {
			if b == 0 {
				words.push(std::mem::take(&mut word));
			} else {
				word.push(b);
			}
		}
		words.push(word);
		words
	}
}

/// Consumers, and how far each reads: `log` records every item a closure sees.
pub fn consumers(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let mut log = Vec::new();
	match kind % 6 {
		0 => {
			let any = data.iter().inspect(|&&b| log.push(b)).any(|&b| b > 0xf0);
			out.push(any as u8);
			let all = data.iter().all(|&b| b != 0);
			out.push(all as u8);
			out.push(data.iter().count() as u8);
			out.push(data.iter().filter(|&&b| b % 2 == 0).count() as u8);
		}
		1 => {
			let found = data.iter().inspect(|&&b| log.push(b)).find(|&&b| b % 7 == 3);
			out.push(found.copied().unwrap_or(0xff));
			let mapped = data
				.iter()
				.find_map(|&b| if b > 0x50 { b.checked_mul(3) } else { None });
			out.push(mapped.unwrap_or(0));
			let position = data.iter().position(|&b| b == 0x40);
			out.push(position.map_or(0xff, |p| p as u8));
			// A zip of slices reads the longer side's closure only up to the shorter's length.
			let short = &data[data.len() / 2..];
			let logged = |b: &u8| *b;
			let pairs = data
				.iter()
				.map(|&b| {
					log.push(b);
					b
				})
				.zip(short)
				.count();
			out.push(pairs as u8);
			let walked = data.iter().map(|&b| {
				log.push(!b);
				b
			});
			for (a, b) in walked.zip(short.iter()) {
				out.push(a ^ b);
			}
			for (a, &b) in data.iter().inspect(|&&b| log.push(b)).map(logged).zip(short.iter()) {
				out.push(a ^ b);
			}
			let mut back = data.iter().map(|&b| b.rotate_free()).zip(short.iter().copied());
			if let Some((a, b)) = back.next_back() {
				out.push(a);
				out.push(b);
			}
			out.push(back.len() as u8);
		}
		2 => {
			out.push(data.iter().copied().max().unwrap_or(0));
			out.push(data.iter().min().copied().unwrap_or(0xff));
			out.push(*data.iter().max_by_key(|&&b| b % 16).unwrap_or(&0xee));
			out.push(*data.iter().min_by_key(|&&b| b / 16).unwrap_or(&0xee));
			out.push(data.iter().last().copied().unwrap_or(0xdd));
			out.push(data.iter().nth(3).copied().unwrap_or(0xcc));
			// `take`'s `nth` and `nth_back` read on past its limit as std's do.
			let mut it = data.iter();
			out.push(it.by_ref().take(data.len() % 3).nth_back(1).copied().unwrap_or(0xbb));
			out.push(it.len() as u8);
			out.push(it.by_ref().take(data.len() % 5).nth(2).copied().unwrap_or(0xbb));
			out.push(it.len() as u8);
		}
		3 => {
			let wide: u64 = data.iter().map(|&b| b as u64 * 0x0101_0101_0101).sum();
			out.extend_from_slice(&wide.to_be_bytes());
			let folded = data
				.iter()
				.fold(0u32, |acc, &b| (acc << 3).wrapping_add(b as u32 ^ acc));
			out.extend_from_slice(&folded.to_be_bytes());
			data.iter().for_each(|&b| log.push(b.rotate_free()));
		}
		4 => {
			// Panics on overflow, as a debug build's `sum` does.
			let total: u8 = data.iter().sum();
			out.push(total);
			let product: u16 = data.iter().map(|&b| b as u16 % 4 + 1).product();
			out.extend_from_slice(&product.to_be_bytes());
		}
		_ => {
			let parsed: Result<Vec<u8>, u8> = data.iter().map(|&b| if b < 0xf0 { Ok(b) } else { Err(b) }).collect();
			match parsed {
				Ok(v) => out.extend_from_slice(&v),
				Err(e) => out.push(e),
			}
			let some: Option<Vec<u16>> = data
				.iter()
				.inspect(|&&b| log.push(b))
				.map(|&b| (b != 0x20).then_some(b as u16))
				.collect();
			out.push(some.map_or(0xff, |v| v.len() as u8));
		}
	}
	out.extend_from_slice(&log);
}

/// `reduce`, `max_by`, and `min_by`, whose comparators log each pair they see, and
/// `Peekable`'s `peek_mut` and `next_if_eq`.
pub fn reducers(data: &[u8], out: &mut Vec<u8>) {
	let mut log = Vec::new();
	// Of keys that tie, `max_by` keeps the last and `min_by` the first.
	let most = data.iter().max_by(|&&a, &&b| {
		log.extend([a, b]);
		(a % 4).cmp(&(b % 4))
	});
	out.push(most.copied().unwrap_or(0xee));
	let least = data.iter().copied().min_by(|a, b| {
		log.extend([*a, *b]);
		(a / 64).cmp(&(b / 64))
	});
	out.push(least.unwrap_or(0xee));
	let folded = data.iter().copied().reduce(|acc, b| {
		log.push(acc);
		acc.wrapping_mul(3) ^ b
	});
	out.push(folded.unwrap_or(0xdd));
	let longest = data
		.iter()
		.map(|&b| vec![b; usize::from(b % 3)])
		.max_by(|a, b| a.len().cmp(&b.len()));
	out.push(longest.map_or(0xcc, |v| v.len() as u8));
	let joined = data.iter().map(|&b| vec![b]).reduce(|mut a, b| {
		a.extend(b);
		a
	});
	out.extend(joined.unwrap_or_default());
	let mut it = data.iter().copied().peekable();
	if let Some(b) = it.peek_mut() {
		*b = b.wrapping_add(1);
	}
	// Written again through a second `peek_mut` of the item already peeked.
	if let Some(b) = it.peek_mut() {
		*b ^= 0x80;
	}
	out.push(it.next().unwrap_or(0xbb));
	// The second byte matches the first only in a repeated input; a mismatch keeps it.
	let first = data.first().copied().unwrap_or(0);
	out.push(it.next_if_eq(&first).unwrap_or(0xaa));
	out.push(it.next().unwrap_or(0xaa));
	out.push(it.peek_mut().is_none() as u8);
	let mut words = data.iter().map(|&b| vec![b]).peekable();
	if let Some(w) = words.peek_mut() {
		w.push(0x99);
	}
	out.extend(words.next().unwrap_or_default());
	out.push(words.next_if_eq(&vec![first]).map_or(0x88, |w| w[0]));
	out.push(words.count() as u8);
	// Items that are places, written through the one `min_by` keeps: the first of the greatest.
	let mut bytes = data.to_vec();
	if let Some(b) = bytes.iter_mut().min_by(|a, b| b.cmp(a)) {
		*b = 0x77;
	}
	out.extend(bytes);
	out.extend(log);
}

trait RotateFree {
	fn rotate_free(self) -> Self;
}

impl RotateFree for u8 {
	fn rotate_free(self) -> u8 {
		(self << 1) | (self >> 7)
	}
}

/// `nth` and `nth_back` through each adaptor, of a source whose reads `by_ref` or
/// `inspect` shows: each reads on past a `take`'s limit, or stops at it, as std's does.
pub fn skips(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let n = data.first().map_or(0, |&b| b as usize % 4);
	let limit = data.get(1).map_or(0, |&b| b as usize % 3);
	let back = kind & 1 == 1;
	// `nth` or `nth_back` of `$it`, as a byte by `$f`.
	macro_rules! skip {
		($it:expr, $f:expr) => {{
			let mut it = $it;
			let item = if back { it.nth_back(n) } else { it.nth(n) };
			out.push(item.map_or(0xee, $f));
		}};
	}
	// `skip!` of `$adapt` over a `by_ref` source, then over an `inspect`ed one, and what each read.
	macro_rules! sources {
		(|$src:ident| $adapt:expr, $f:expr) => {{
			let mut rest = data.iter();
			{
				let $src = rest.by_ref();
				skip!($adapt, $f);
			}
			out.push(rest.len() as u8);
			let mut log = Vec::new();
			{
				let $src = data.iter().inspect(|&&b| log.push(b));
				skip!($adapt, $f);
			}
			out.extend_from_slice(&log);
		}};
	}
	match (kind / 2) % 14 {
		0 => sources!(|src| src.take(limit).fuse(), |&b| b),
		1 => sources!(|src| src.take(limit + 1).skip(1), |&b| b),
		2 => sources!(|src| src.take(limit).chain([0x11].iter()), |&b| b),
		3 => sources!(|src| [0x11].iter().chain(src.take(limit)), |&b| b),
		4 => sources!(|src| src.take(limit + 1).step_by(2), |&b| b),
		5 => sources!(|src| src.take(limit).peekable(), |&b| b),
		6 => sources!(
			|src| {
				let mut it = src.take(limit + 1).peekable();
				it.peek();
				it
			},
			|&b| b
		),
		7 => sources!(|src| std::iter::once(src.take(limit)).flatten(), |&b| b),
		8 => sources!(|src| src.take(limit).copied(), |b| b),
		9 => sources!(|src| src.take(limit).cloned(), |b| b),
		10 => sources!(|src| src.take(limit).enumerate(), |(i, &b)| b ^ i as u8),
		11 => sources!(|src| src.take(limit).rev(), |&b| b),
		// `fuse` advances by `next`, where `take` would read on past its limit.
		12 => sources!(|src| [0x11].iter().chain(src.take(limit).fuse()), |&b| b),
		_ => {
			// Copies that JS makes by a function, which std's `copied` skips through.
			let pairs: Vec<(u8, u8)> = data.iter().map(|&b| (b, !b)).collect();
			let mut rest = pairs.iter();
			skip!(rest.by_ref().take(limit).copied(), |(a, b)| a ^ b);
			out.push(rest.len() as u8);
			let arrays: Vec<[u8; 2]> = data.iter().map(|&b| [b, !b]).collect();
			let mut rest = arrays.iter();
			skip!(rest.by_ref().take(limit).copied(), |[a, _]| a);
			out.push(rest.len() as u8);
		}
	}
}

/// An order whose `cmp` leans on its first operand, an odd one less than any, so which
/// operand std's `min_by_key`, `max_by_key`, `min`, and `max` put first shows.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Tilt(u8);

impl PartialOrd for Tilt {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Tilt {
	fn cmp(&self, other: &Self) -> Ordering {
		match self.0 % 2 {
			1 => Ordering::Less,
			_ => self.0.cmp(&other.0),
		}
	}
}

/// `min_by_key` and `max_by_key` by keys that are tuples, `Reverse`s, `Option`s, and the
/// crate's own orders, `min` and `max` of such items, and `rposition` and `rfind`.
pub fn ranked(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	// Where the item found was, by its index.
	let put = |out: &mut Vec<u8>, found: Option<(usize, u8)>| out.push(found.map_or(0xff, |(i, _)| i as u8));
	let tier = |b: u8| match b % 3 {
		0 => None,
		1 => Some(Reverse(b / 3 % 4)),
		_ => Some(Reverse(0)),
	};
	let items = || data.iter().copied().enumerate();
	let tilts: Vec<Tilt> = data.iter().map(|&b| Tilt(b % 8)).collect();
	match kind % 4 {
		0 => {
			// Of equal keys, `min_by_key` keeps the first and `max_by_key` the last.
			put(out, items().min_by_key(|&(_, b)| (Reverse(b % 4), tier(b))));
			put(out, items().max_by_key(|&(_, b)| (Reverse(b % 4), tier(b))));
			put(out, items().filter(|&(_, b)| b > 0x40).max_by_key(|&(_, b)| tier(b)));
			put(out, items().min_by_key(|&(_, b)| (b % 2 == 0, Reverse((b % 3) as u64))));
		}
		1 => {
			// By the crate's own order, and by keys that borrow from the items.
			put(out, items().min_by_key(|&(_, b)| Tilt(b % 8)));
			put(out, items().max_by_key(|&(_, b)| (b % 2 == 0, Tilt(b % 8))));
			let first = |t: Option<&Tilt>| t.map_or(0xff, |t| t.0);
			out.push(first(tilts.iter().min_by_key(|&t| Reverse(Some(t)))));
			out.push(first(tilts.iter().max_by_key(|&t| (Reverse(t), t.0 / 2))));
		}
		2 => {
			// Items that are tuples, and the crate's own.
			let pair = |b: u8| (b % 4, Reverse(Some(b / 4).filter(|b| b % 3 != 0)));
			for best in [data.iter().map(|&b| pair(b)).max(), data.iter().map(|&b| pair(b)).min()] {
				match best {
					Some((a, Reverse(b))) => out.extend_from_slice(&[a, b.unwrap_or(0xff)]),
					None => out.push(0xfe),
				}
			}
			out.push(tilts.iter().copied().max().map_or(0xff, |t| t.0));
			out.push(tilts.iter().min().map_or(0xff, |t| t.0));
		}
		_ => {
			// From the back, counting from the front, of what is left.
			out.push(data.iter().rposition(|&b| b % 3 == 0).map_or(0xff, |i| i as u8));
			out.push(data.iter().rfind(|&&b| b > 0x80).copied().unwrap_or(0));
			let mut seen = Vec::new();
			let mut rest = tilts.iter();
			rest.next();
			let found = rest.rposition(|t| {
				seen.push(t.0);
				t.0 == 2
			});
			out.push(found.map_or(0xff, |i| i as u8));
			out.extend_from_slice(&seen);
			out.push(rest.len() as u8);
			out.push(rest.rfind(|t| t.0 > 3).map_or(0xff, |t| t.0));
			out.push(rest.len() as u8);
		}
	}
}

/// Ranges as values and as iterators.
pub fn ranges(data: &[u8], out: &mut Vec<u8>) {
	let a = data.first().copied().unwrap_or(0);
	let b = data.get(1).copied().unwrap_or(0);
	let range = a..b;
	out.push(range.start);
	out.push(range.end);
	out.push(range.is_empty() as u8);
	out.push(range.contains(&0x40) as u8);
	let mut copy = range.clone();
	out.push(copy.next().unwrap_or(0xff));
	out.push(range.len() as u8);
	for i in range.rev().step_by(3) {
		out.push(i);
	}
	for i in (a..b).skip(1).step_by(4) {
		out.push(i);
	}
	out.push((a..b).nth(5).unwrap_or(0xee));
	// `..=` at the top of `u8`, which ends without overflowing.
	let top: Vec<u8> = (a.max(250)..=255).collect();
	out.extend_from_slice(&top);
	let inclusive = a..=b;
	out.push(*inclusive.start());
	out.push(*inclusive.end());
	out.push(inclusive.contains(&b) as u8);
	// Of constants, which is itself a constant.
	const LOW: u64 = 0x20;
	const HIGH: u64 = 0x7f;
	out.push((LOW..=HIGH).contains(&u64::from(a)) as u8);
	// `nth` past the end, or past the type's last value, leaves the range as std's does.
	let mut near = a.max(200)..=255;
	out.push(near.nth(b as usize).unwrap_or(0xee));
	out.push(*near.start());
	out.push(near.is_empty() as u8);
	out.push(near.contains(&255) as u8);
	out.push(near.contains(&100) as u8);
	let mut low = 0..=b;
	out.push(low.nth_back(a as usize % 16).unwrap_or(0xee));
	out.push(*low.end());
	out.push(low.contains(&(b / 2)) as u8);
	let mut reversed = a..=b;
	out.push(reversed.by_ref().rev().nth(a as usize).unwrap_or(0xee));
	out.push(*reversed.end());
	out.push(reversed.contains(&a) as u8);
	let mut counted = (a..=b).enumerate();
	let (i, n) = counted.nth(b as usize / 4).unwrap_or((0xdd, 0xdd));
	out.push(i as u8);
	out.push(n);
	out.push(counted.next().map_or(0xcc, |(i, _)| i as u8));
	let mut stepped = a..=b;
	out.push(stepped.by_ref().step_by(3).count() as u8);
	out.push(*stepped.start());
	let signed = (a as i8)..=(b as i8);
	out.extend(signed.rev().map(|i| i as u8));
	let mut wide = (a as u64)..(b as u64 * 3);
	out.push(wide.by_ref().map(|n| n % 5).filter(|&n| n == 0).count() as u8);
	out.push(wide.is_empty() as u8);
	let total: u64 = (0..a as u64).sum();
	out.extend_from_slice(&total.to_be_bytes());
	// Wide ranges stepping past 2^53, where a number becomes limbs, and up to the maximum.
	let edge = (1u64 << 53) - 2 + a as u64 % 4;
	let mut past = edge..edge + 4;
	out.extend_from_slice(&past.nth(b as usize % 3).unwrap_or(0).to_be_bytes());
	out.extend_from_slice(&past.next_back().unwrap_or(0).to_be_bytes());
	out.extend_from_slice(&(edge..=edge + 3).nth(b as usize % 5).unwrap_or(0).to_be_bytes());
	let top: u64 = (u64::MAX - a as u64 % 3..=u64::MAX).map(|n| n % 7).sum();
	out.extend_from_slice(&top.to_be_bytes());
	let most = (edge..=edge + 2).rev().max_by_key(|&n| n).unwrap_or(0);
	out.extend_from_slice(&most.to_be_bytes());
	let least = (-(1i64 << 53) - 1..-(1i64 << 53) + 1).min().unwrap_or(0);
	out.extend_from_slice(&least.to_be_bytes());
	let steps: Vec<u32> = (0..48).step_by(8).rev().collect();
	for s in steps {
		out.push(s as u8);
	}
	updates(a, b, data, out);
}

/// Struct update syntax on std's ranges: the fields written run before the base, and
/// the rest are read from it.
fn updates(a: u8, b: u8, data: &[u8], out: &mut Vec<u8>) {
	use std::ops::Range;
	let mut base = a..b;
	let made = Range {
		end: {
			out.push(0xd0);
			b / 2
		},
		..{
			out.push(0xd1);
			base.clone()
		}
	};
	let whole = Range { ..base.clone() };
	out.extend_from_slice(&[made.start, made.end, whole.start, whole.end]);
	// The loop's end is read once, though the base it came from changes.
	for i in (Range { start: a / 2, ..base }) {
		base.end = base.end.wrapping_add(1);
		out.push(i);
	}
	out.push(base.end);
	let len = data.len();
	let (to, from) = ((b as usize).min(len), (a as usize).min(len));
	let all = 0..len;
	// The sliced value, the fields written, then the base.
	let sliced = &{
		out.push(0xd2);
		data
	}[Range {
		start: {
			out.push(0xd3);
			from
		},
		..{
			out.push(0xd4);
			all.clone()
		}
	}];
	out.extend_from_slice(sliced);
	let full = &data[std::ops::RangeFull {
		..{
			out.push(0xd5);
			std::ops::RangeFull
		}
	}];
	out.push(full.len() as u8);
	let full = &data[{
		out.push(0xd6);
		..
	}];
	out.push(full.len() as u8);
	let full = &data[full_range(out)];
	out.push(full.len() as u8);
	let full = &data[..];
	out.push(full.len() as u8);
	let mut copied = data.to_vec();
	copied.copy_within(Range { end: to, ..all }, 0);
	out.extend_from_slice(&copied);
}

/// `..`, logging that it was made.
#[inline(never)]
fn full_range(out: &mut Vec<u8>) -> std::ops::RangeFull {
	out.push(0xd7);
	..
}

/// Iterators of `Option`, which keep each `None` apart from their end.
pub fn options(data: &[u8], out: &mut Vec<u8>) {
	let maybe = |b: &u8| if b % 3 == 0 { None } else { Some(*b) };
	let all: Vec<Option<u8>> = data.iter().map(maybe).collect();
	out.push(all.len() as u8);
	for item in data.iter().map(maybe) {
		match item {
			Some(b) => out.push(b),
			None => out.push(0),
		}
	}
	let flat: Vec<u8> = data.iter().map(maybe).flatten().collect();
	out.extend_from_slice(&flat);
	let somes = all.iter().filter(|o| o.is_some()).count();
	out.push(somes as u8);
	out.push(second(data.iter().map(maybe)).unwrap_or(0xcd));
	let first = data.first().copied();
	for b in first.iter().chain(std::iter::once(&0x99)) {
		out.push(*b);
	}
	out.extend(first.into_iter().chain(std::iter::empty()));
	let mut n = 0u8;
	let counted: Vec<u8> = std::iter::from_fn(|| {
		n += 1;
		(n < 4).then_some(n)
	})
	.collect();
	out.extend_from_slice(&counted);
	// Counted through a borrow, which leaves a `Chain` to read on past a gap.
	let mut k = 0u8;
	let mut gappy = std::iter::empty().chain(std::iter::from_fn(|| {
		k += 1;
		(k != 2).then_some(k)
	}));
	out.push(gappy.by_ref().count() as u8);
	out.push(gappy.next().unwrap_or(0xee));
	// `successors` calls its closure on each item as it yields it, and never again once
	// it returned `None`.
	let mut calls = Vec::new();
	let halving = |b: &u8, calls: &mut Vec<u8>| {
		calls.push(*b);
		b.checked_sub(1).map(|b| b / 2)
	};
	let first = data.first().copied();
	let taken: Vec<u8> = std::iter::successors(first, |b| halving(b, &mut calls))
		.take(data.len() % 4)
		.collect();
	out.extend_from_slice(&taken);
	out.extend_from_slice(&calls);
	let count = std::iter::successors(first, |b| halving(b, &mut calls)).count();
	out.push(count as u8);
	out.extend_from_slice(&calls);
	// The crate's own iterator, whose items are options.
	let gaps = Gaps { data, at: 0 };
	for gap in gaps {
		out.push(gap.map_or(0xaa, |b| b));
	}
}

/// The second item of an iterator of options, through `?`, which tests for its end.
fn second(mut items: impl Iterator<Item = Option<u8>>) -> Option<u8> {
	items.next()?;
	let item = items.next()?;
	Some(item.unwrap_or(0xab))
}

/// Every byte, or `None` where it is zero.
struct Gaps<'a> {
	data: &'a [u8],
	at: usize,
}

impl Iterator for Gaps<'_> {
	type Item = Option<u8>;

	fn next(&mut self) -> Option<Option<u8>> {
		let &b = self.data.get(self.at)?;
		self.at += 1;
		if b == 0 {
			return Some(None);
		}
		Some(Some(b))
	}
}

/// A countdown: the crate's own iterator.
struct Countdown {
	left: u8,
	step: u8,
}

impl Iterator for Countdown {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		match self.left {
			0 => None,
			left => {
				self.left = left.saturating_sub(self.step);
				Some(left)
			}
		}
	}
}

/// The even bytes, read through another iterator, whose `None` ends this one.
struct Evens<'a> {
	inner: std::iter::Copied<std::slice::Iter<'a, u8>>,
}

impl Iterator for Evens<'_> {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		loop {
			let b = self.inner.next()?;
			if b % 2 == 0 {
				return Some(b);
			}
		}
	}
}

/// The crate's own iterators, through `for`, adaptors, and `next`.
pub fn own(data: &[u8], out: &mut Vec<u8>) {
	let left = data.first().copied().unwrap_or(0);
	let step = data.get(1).copied().unwrap_or(1).max(1);
	for n in (Countdown { left, step }) {
		out.push(n);
	}
	let total: u64 = Countdown { left, step }.map(|n| n as u64 * 1000 / 7).sum();
	out.extend_from_slice(&total.to_be_bytes());
	let mut down = Countdown { left, step };
	while let Some(n) = down.next() {
		if n % 5 == 0 {
			break;
		}
	}
	out.push(down.left);
	let evens: Vec<u8> = Evens {
		inner: data.iter().copied(),
	}
	.collect();
	out.extend_from_slice(&evens);
	let mut by_ref = Evens {
		inner: data[data.len().min(1)..].iter().copied(),
	};
	let firsts: Vec<u8> = by_ref.by_ref().take(2).collect();
	out.extend_from_slice(&firsts);
	out.push(by_ref.count() as u8);
	// Two instances of one generic iterator, each with its own `next` on one class.
	let halves: Vec<u8> = Halving { value: left }.collect();
	out.extend_from_slice(&halves);
	let total: u64 = Halving {
		value: left as u64 * 0x1_0000_0001,
	}
	.sum();
	out.extend_from_slice(&total.to_be_bytes());
}

/// A value halved until it is zero: generic, with an `impl Iterator` per instance.
struct Halving<T> {
	value: T,
}

impl Iterator for Halving<u8> {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		let value = self.value;
		self.value /= 2;
		(value != 0).then_some(value)
	}
}

impl Iterator for Halving<u64> {
	type Item = u64;

	fn next(&mut self) -> Option<u64> {
		match self.value {
			0 => None,
			value => {
				self.value = value >> 1;
				Some(value)
			}
		}
	}
}

/// The last two bytes of `data`, as a `slice::Iter` made from the second argument: a
/// `for` must iterate it, not the first.
fn tail<'a>(_head: &'a [u8], data: &'a [u8]) -> std::slice::Iter<'a, u8> {
	data[data.len().saturating_sub(2)..].iter()
}

/// The odd bytes, from a function returning `impl Iterator`.
fn odds(data: &[u8]) -> impl Iterator<Item = u8> + '_ {
	data.iter().copied().filter(|b| b % 2 == 1)
}

/// Iterators as values: returned, stored, and printed.
pub fn values(data: &[u8], out: &mut Vec<u8>) {
	for &b in tail(&[1, 2, 3], data) {
		out.push(b);
	}
	let mut held = Held { it: data.iter() };
	out.push(held.skip_one());
	for b in odds(data) {
		out.push(b);
	}
	let printed = format!("{:?}|{:#?}", Doubled(data), Doubled(&data[..data.len().min(2)]));
	out.extend_from_slice(printed.as_bytes());
	// Clones that change leave what they cloned alone.
	let tags: Vec<Tag> = data
		.iter()
		.map(|&b| Tag {
			bytes: [b, !b].to_vec(),
		})
		.collect();
	let mut cloned: Vec<Tag> = tags.iter().cloned().collect();
	for tag in cloned.iter_mut() {
		tag.bytes.push(0x77);
	}
	for tag in tags.iter().chain(cloned.iter()) {
		out.extend_from_slice(&tag.bytes);
	}
	// A `Copy` type's own `Clone`, which `cloned` calls and `copied` does not.
	let bumps: Vec<Bumped> = data.iter().map(|&b| Bumped(b)).collect();
	out.extend(bumps.iter().cloned().map(|b| b.0));
	out.extend(bumps.iter().copied().map(|b| b.0));
	// A derived `Clone` over a type parameter clones the field, by its own `Clone`.
	let wrapped: Vec<Wrapper<Bumped>> = data.iter().map(|&b| Wrapper(Bumped(b))).collect();
	out.extend(wrapped.iter().cloned().map(|w| w.0.0));
}

#[derive(Clone, Copy)]
struct Wrapper<T>(T);

/// A `Copy` type whose `Clone` does more than copy.
#[derive(Copy)]
pub struct Bumped(pub u8);

impl Clone for Bumped {
	fn clone(&self) -> Self {
		Bumped(self.0.wrapping_add(1))
	}
}

/// A `Clone` that is not `Copy`, which `cloned` calls for each item.
#[derive(Clone)]
struct Tag {
	bytes: Vec<u8>,
}

/// An iterator in a field.
struct Held<'a> {
	it: std::slice::Iter<'a, u8>,
}

impl Held<'_> {
	fn skip_one(&mut self) -> u8 {
		self.it.next();
		self.it.next().copied().unwrap_or(0xbb)
	}
}

/// Each byte doubled, as a `Debug` list built from an iterator.
struct Doubled<'a>(&'a [u8]);

impl fmt::Debug for Doubled<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_list().entries(self.0.iter().map(|&b| b as u16 * 2)).finish()
	}
}

/// What std's `Option`-returning calls give, returned as is: an `Option`, whose `None`
/// stays apart from an item's own.
pub fn returned(data: &[u8], out: &mut Vec<u8>) {
	let maybe = |b: &u8| if b % 3 == 0 { None } else { Some(*b) };
	let push = |b: Option<u8>, out: &mut Vec<u8>| out.push(b.map_or(0xaa, |b| b));
	let nested = |b: Option<Option<u8>>, out: &mut Vec<u8>| match b {
		None => out.push(0xbb),
		Some(None) => out.push(0xcc),
		Some(Some(b)) => out.push(b),
	};
	let n = data.first().map_or(0, |&b| usize::from(b % 4));
	push(first(data.iter().copied()), out);
	push(back(data.iter().copied()), out);
	push(nth(data.iter().copied(), n), out);
	push(last(data.iter().copied()), out);
	nested(first(data.iter().map(maybe)), out);
	nested(back(data.iter().map(maybe)), out);
	nested(first(Gaps { data, at: n }), out);
	push(first(Countdown { left: n as u8, step: 1 }), out);
	let text = std::str::from_utf8(data).unwrap_or("é");
	out.extend(first(text.chars()).map_or(0, u32::from).to_le_bytes());
	let mut copied = data.iter().copied();
	push(by_mut(&mut copied), out);
	let mut gaps = Gaps { data, at: 0 };
	nested(by_mut(&mut gaps), out);
	nested(by_mut(&mut gaps), out);
	let mut it = data.iter();
	push(borrowed(&mut it).copied(), out);
	push(borrowed(&mut it).copied(), out);
	push(second_of(data).copied(), out);
	push(from_one(n as u8), out);
	let mut peekable = data.iter().copied().peekable();
	push(peeked(&mut peekable).copied(), out);
	push(at(data, n).copied(), out);
	let maybes: Vec<Option<u8>> = data.iter().map(maybe).collect();
	nested(at(&maybes, n).copied(), out);
}

fn first<I: Iterator>(mut it: I) -> Option<I::Item> {
	it.next()
}

fn back<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
	it.next_back()
}

fn nth<I: Iterator>(mut it: I, n: usize) -> Option<I::Item> {
	it.nth(n)
}

fn last<I: Iterator>(it: I) -> Option<I::Item> {
	it.last()
}

/// Through a `&mut` to the iterator.
fn by_mut<I: Iterator>(it: &mut I) -> Option<I::Item> {
	it.next()
}

/// Through a borrow of a std iterator the function is not generic over.
fn borrowed<'a>(it: &mut std::slice::Iter<'a, u8>) -> Option<&'a u8> {
	it.next()
}

/// Of a slice's iterator the function makes.
fn second_of(data: &[u8]) -> Option<&u8> {
	let mut it = data.iter();
	it.next();
	it.next()
}

/// Of a small range the function makes.
fn from_one(n: u8) -> Option<u8> {
	let mut range = 1..n;
	range.next()
}

fn peeked<I: Iterator>(it: &mut std::iter::Peekable<I>) -> Option<&I::Item> {
	it.peek()
}

fn at<T>(items: &[T], i: usize) -> Option<&T> {
	items.get(i)
}
