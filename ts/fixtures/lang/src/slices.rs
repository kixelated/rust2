//! A slice's `windows` and `chunks`, of bytes and of other values, read from both ends,
//! skipped through, and counted, at sizes past the slice's length; and `Vec::drain` of
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
