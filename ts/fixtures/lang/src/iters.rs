//! Iterators: std's adaptors and consumers, ranges, iterators of `Option`, and the
//! crate's own `impl Iterator`. Closures with effects log each call, so laziness and
//! where a consumer stops are part of the output.

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

trait RotateFree {
	fn rotate_free(self) -> Self;
}

impl RotateFree for u8 {
	fn rotate_free(self) -> u8 {
		(self << 1) | (self >> 7)
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
	let steps: Vec<u32> = (0..48).step_by(8).rev().collect();
	for s in steps {
		out.push(s as u8);
	}
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
