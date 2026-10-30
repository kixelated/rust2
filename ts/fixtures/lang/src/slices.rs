//! A slice's `windows` and each of its chunks, of bytes and of other values, read from
//! both ends, skipped through, and counted, at sizes past the slice's length, with what
//! is written through a mutable chunk landing in the slice; and `Vec::drain` of
//! each kind of range, read from both ends, dropped part way, and forgotten, of values
//! that log when they drop.

use std::cell::RefCell;

thread_local! {
	static LOG: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Puts which items dropped since the last flush.
fn flush(out: &mut Vec<u8>) {
	LOG.with_borrow_mut(|log| {
		out.extend_from_slice(log);
		log.clear();
	});
}

/// Logs its tag as it drops.
struct Item {
	tag: u8,
}

impl Drop for Item {
	fn drop(&mut self) {
		LOG.with_borrow_mut(|log| log.push(self.tag));
	}
}

/// A digest of a part, which tells parts of different lengths or contents apart.
fn digest(part: &[u8]) -> u8 {
	part.iter()
		.fold(part.len() as u8, |acc, &b| acc.wrapping_mul(31).wrapping_add(b))
}

fn digest_words(part: &[u16]) -> u8 {
	part.iter()
		.fold(part.len() as u8, |acc, &w| acc.wrapping_mul(31) ^ (w >> 3) as u8)
}

/// Each way of reading `$parts`, an expression that makes the iterator anew.
macro_rules! read {
	($parts:expr, $digest:expr, $out:expr) => {{
		let out: &mut Vec<u8> = $out;
		out.extend($parts.map($digest));
		out.extend($parts.rev().map($digest));
		let mut it = $parts;
		out.push(it.len() as u8);
		out.extend(it.next().map($digest));
		out.extend(it.next_back().map($digest));
		out.extend(it.nth(1).map($digest));
		out.extend(it.nth_back(1).map($digest));
		out.push(it.len() as u8);
		out.extend(it.map($digest));
		out.extend($parts.last().map($digest));
		out.push($parts.count() as u8);
		out.extend($parts.skip(2).step_by(2).map($digest));
	}};
}

/// `windows` or `chunks`, as `kind` picks, of a size it picks too, which is 0 one time in
/// six, which panics.
pub fn parts(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let size = (kind / 2 % 6) as usize;
	let words: Vec<u16> = data.iter().map(|&b| b as u16 * 257).collect();
	match kind % 2 {
		0 => {
			read!(data.windows(size), digest, out);
			read!(words.windows(size), digest_words, out);
			// A window kept past the next, and compared with it.
			let mut steps = data.windows(size);
			if let (Some(a), Some(b)) = (steps.next(), steps.next()) {
				out.extend([(a == b) as u8, a[0], b[size - 1]]);
			}
		}
		_ => {
			read!(data.chunks(size), digest, out);
			read!(words.chunks(size), digest_words, out);
			// The remainder, which the last chunk is.
			out.extend(data.chunks(size).next_back().map(|c| c.len() as u8));
			let rows: Vec<u16> = words.chunks(size).map(|c| c.iter().sum::<u16>() >> 2).collect();
			out.extend(rows.iter().map(|&r| r as u8));
		}
	}
}

/// An element kept as an object, which a mutable chunk writes whole and by field.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Pair {
	a: u8,
	b: u16,
}

/// Writes through one mutable chunk, the `i`th, as each element type allows.
fn scribble(i: usize, chunk: &mut [u8]) {
	let n = chunk.len();
	chunk[0] = chunk[0].wrapping_add(i as u8);
	chunk.swap(0, n - 1);
	if i % 2 == 1 {
		chunk.reverse();
	}
	for b in chunk.iter_mut() {
		*b ^= 0x5a;
	}
}

fn scribble_words(i: usize, chunk: &mut [u16]) {
	let n = chunk.len();
	chunk[n - 1] = chunk[n - 1].wrapping_mul(3).wrapping_add(i as u16);
	chunk.swap(0, n / 2);
	if let Some(w) = chunk.first_mut() {
		*w ^= 0x1234;
	}
	if i % 3 == 2 {
		for w in chunk.iter_mut() {
			*w = i as u16;
		}
	}
	// A chunk of a chunk writes through to the slice too.
	for (j, inner) in chunk.chunks_mut(2).enumerate() {
		inner[0] = inner[0].wrapping_add(j as u16);
	}
}

fn scribble_pairs(i: usize, chunk: &mut [Pair]) {
	let n = chunk.len();
	chunk[0].a = chunk[0].a.wrapping_add(i as u8);
	chunk[n - 1] = Pair {
		a: chunk[n - 1].b as u8,
		b: chunk[0].a as u16,
	};
	chunk.sort();
	chunk.swap(0, n / 2);
}

fn digest_pairs(part: &[Pair]) -> u8 {
	part.iter()
		.fold(part.len() as u8, |acc, p| acc.wrapping_mul(31) ^ p.a ^ (p.b >> 4) as u8)
}

/// Each mutable chunk of `$data`, written as `$scribble` writes the `i`th; then each of a
/// pass that keeps every chunk at once, written back to front; then `$data` whole.
macro_rules! write {
	($data:expr, $chunks:ident, $size:expr, $scribble:expr, $digest:expr, $out:expr) => {{
		let out: &mut Vec<u8> = $out;
		for (i, chunk) in $data.$chunks($size).enumerate() {
			$scribble(i, chunk);
		}
		out.push($digest(&$data));
		let mut held: Vec<_> = $data.$chunks($size).collect();
		for (i, chunk) in held.iter_mut().enumerate().rev() {
			$scribble(i + 7, chunk);
		}
		out.extend(held.iter().map(|c| $digest(c)));
		// From both ends and by `nth`, writing each as it comes.
		let mut it = $data.$chunks($size);
		out.push(it.len() as u8);
		if let Some(c) = it.next_back() {
			$scribble(1, c);
		}
		if let Some(c) = it.nth(1) {
			$scribble(2, c);
		}
		if let Some(c) = it.nth_back(1) {
			$scribble(3, c);
		}
		out.push(it.len() as u8);
		if let Some(c) = it.last() {
			$scribble(4, c);
		}
		out.push($data.$chunks($size).count() as u8);
		out.push($digest(&$data));
	}};
}

/// `chunks_exact`, `rchunks`, and `rchunks_exact`, and the mutable forms of those and
/// `chunks`, as `kind` picks, of a size it picks too, which is 0 one time in six, which
/// panics.
pub fn chunked(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let size = (kind / 8 % 6) as usize;
	let mut bytes = data.to_vec();
	let mut words: Vec<u16> = data.iter().map(|&b| b as u16 * 257).collect();
	let mut pairs: Vec<Pair> = data.iter().map(|&b| Pair { a: b, b: b as u16 * 3 }).collect();
	match kind % 8 {
		0 => {
			read!(data.chunks_exact(size), digest, out);
			read!(words.chunks_exact(size), digest_words, out);
			// The remainder, which no part holds, stays as the parts are taken.
			let mut it = data.chunks_exact(size);
			out.push(digest(it.remainder()));
			it.next();
			it.next_back();
			out.push(digest(it.remainder()));
			out.push(digest_words(words.chunks_exact(size).remainder()));
		}
		1 => {
			read!(data.rchunks(size), digest, out);
			read!(words.rchunks(size), digest_words, out);
			// The last part is the front of the slice, the short one.
			out.extend(data.rchunks(size).next_back().map(|c| c.len() as u8));
		}
		2 => {
			read!(data.rchunks_exact(size), digest, out);
			read!(words.rchunks_exact(size), digest_words, out);
			let mut it = data.rchunks_exact(size);
			out.push(digest(it.remainder()));
			it.nth(1);
			out.push(digest(it.remainder()));
			out.push(digest_words(words.rchunks_exact(size).remainder()));
		}
		3 => {
			write!(bytes, chunks_mut, size, scribble, digest, out);
			write!(words, chunks_mut, size, scribble_words, digest_words, out);
			write!(pairs, chunks_mut, size, scribble_pairs, digest_pairs, out);
		}
		4 => {
			write!(bytes, chunks_exact_mut, size, scribble, digest, out);
			write!(words, chunks_exact_mut, size, scribble_words, digest_words, out);
			write!(pairs, chunks_exact_mut, size, scribble_pairs, digest_pairs, out);
			// The remainder, written through after some parts are taken.
			let mut it = bytes.chunks_exact_mut(size);
			it.next();
			let rest = it.into_remainder();
			rest.reverse();
			let mut it = words.chunks_exact_mut(size);
			it.next_back();
			for w in it.into_remainder() {
				*w = w.wrapping_add(1);
			}
			if let Some(p) = pairs.chunks_exact_mut(size).into_remainder().first_mut() {
				*p = Pair { a: 0xee, b: 0xeeee };
			}
		}
		5 => {
			write!(bytes, rchunks_mut, size, scribble, digest, out);
			write!(words, rchunks_mut, size, scribble_words, digest_words, out);
			write!(pairs, rchunks_mut, size, scribble_pairs, digest_pairs, out);
		}
		6 => {
			write!(bytes, rchunks_exact_mut, size, scribble, digest, out);
			write!(words, rchunks_exact_mut, size, scribble_words, digest_words, out);
			write!(pairs, rchunks_exact_mut, size, scribble_pairs, digest_pairs, out);
			let rest = bytes.rchunks_exact_mut(size).into_remainder();
			if let Some(b) = rest.last_mut() {
				*b = 0xee;
			}
			let mut it = words.rchunks_exact_mut(size);
			it.next();
			for w in it.into_remainder() {
				*w = 7;
			}
			pairs.rchunks_exact_mut(size).into_remainder().reverse();
		}
		// Chunks of a mutable slice that is itself part of the vector.
		_ => {
			let tail = &mut bytes[data.len() / 3..];
			for (i, chunk) in tail.rchunks_mut(size).enumerate() {
				scribble(i, chunk);
			}
			for (i, chunk) in words.as_mut_slice().chunks_exact_mut(size).rev().enumerate() {
				scribble_words(i, chunk);
			}
		}
	}
	out.extend(&bytes);
	out.extend(words.iter().map(|&w| (w ^ w >> 8) as u8));
	out.push(digest_pairs(&pairs));
}

/// `Vec::drain` of a range from the first two bytes, which may be out of bounds or
/// backwards, which panics, of the rest as items, bytes, and integers: what it yields, and
/// what each drops and keeps as the drain drops, or as it is forgotten.
pub fn drains(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let [a, b, data @ ..] = data else {
		return;
	};
	let reach = data.len() + 2;
	let (a, b) = (*a as usize % reach, *b as usize % reach);
	let mut items: Vec<Item> = data.iter().map(|&tag| Item { tag }).collect();
	let mut bytes = data.to_vec();
	let mut words: Vec<u32> = data.iter().map(|&b| b as u32 + 1000).collect();
	match kind % 8 {
		0 => {
			for x in items.drain(a..b) {
				out.push(x.tag);
				flush(out);
			}
			out.extend(bytes.drain(a..b));
			out.extend(words.drain(a..b).map(|w| w as u8));
		}
		// The rest of the range drops front to back, before the tail moves back.
		1 => {
			let mut drain = items.drain(a..=b);
			out.extend(drain.next().map(|x| x.tag));
			out.extend(drain.next_back().map(|x| x.tag));
			out.push(drain.len() as u8);
			drop(drain);
			flush(out);
			let mut drain = bytes.drain(a..=b);
			out.extend(drain.next_back());
			drop(drain);
			let mut drain = words.drain(a..=b);
			out.extend(drain.next().map(|w| w as u8));
		}
		2 => {
			let taken: Vec<Item> = items.drain(..b).collect();
			out.extend(taken.iter().map(|x| x.tag));
			drop(taken);
			let taken: Vec<u8> = bytes.drain(..b).collect();
			out.extend(taken);
			words.drain(..b);
		}
		// What `nth` and `nth_back` skip drops as they pass it.
		3 => {
			let mut drain = items.drain(a..);
			out.extend(drain.nth(1).map(|x| x.tag));
			flush(out);
			out.extend(drain.nth_back(1).map(|x| x.tag));
			flush(out);
			out.push(drain.len() as u8);
			drop(drain);
			out.extend(bytes.drain(a..).nth(2));
			out.extend(words.drain(a..).rev().map(|w| w as u8));
		}
		4 => {
			out.extend(items.drain(..).last().map(|x| x.tag));
			out.extend(bytes.drain(..).last());
			out.push(words.drain(..).count() as u8);
		}
		// Forgotten, a drain leaves the vector what came before the range, and leaks the rest.
		5 => {
			let mut drain = items.drain(a..b);
			out.extend(drain.next().map(|x| x.tag));
			std::mem::forget(drain);
			let mut drain = bytes.drain(a..b);
			out.extend(drain.next());
			std::mem::forget(drain);
			std::mem::forget(words.drain(a..b));
		}
		6 => out.push(items.drain(a..b).count() as u8),
		_ => {
			let mut kept = vec![Item { tag: 0xee }];
			kept.extend(items.drain(a..b).filter(|x| x.tag % 2 == 0));
			flush(out);
			out.extend(kept.iter().map(|x| x.tag));
			let mut other = vec![0xee];
			other.extend(bytes.drain(a..b).rev());
			out.extend(other);
		}
	}
	flush(out);
	out.extend(items.iter().map(|x| x.tag));
	out.push(items.len() as u8);
	out.extend(&bytes);
	out.extend(words.iter().map(|&w| w as u8));
	items.push(Item { tag: 0xff });
	drop(items);
	flush(out);
}
