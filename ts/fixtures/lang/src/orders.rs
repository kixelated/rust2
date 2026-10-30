//! Ordering in std: `cmp`'s `max` and `min` and their `_by` and `_by_key` forms, the
//! `is_sorted` family, and `BinaryHeap`, of a type that logs each comparison std makes of
//! it and each drop, so which tests run, in what order, which of equal values comes out,
//! and when each value drops all show.

use std::cell::RefCell;
use std::cmp::{self, Ordering, Reverse};
use std::collections::BinaryHeap;
use std::collections::binary_heap::PeekMut;

thread_local! {
	static LOG: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn log(entry: [u8; 3]) {
	LOG.with_borrow_mut(|log| log.extend(entry));
}

/// Puts what was logged since the last flush.
fn flush(out: &mut Vec<u8>) {
	LOG.with_borrow_mut(|log| {
		out.extend_from_slice(log);
		log.clear();
	});
}

/// Ordered by `key` alone, so `tag` tells equal ones apart. Each comparison logs which it
/// is and the two tags, and a drop logs the tag.
struct Item {
	key: u8,
	tag: u8,
}

fn item(tag: usize, key: u8) -> Item {
	Item {
		key: key % 8,
		tag: tag as u8,
	}
}

impl Drop for Item {
	fn drop(&mut self) {
		log([b'd', self.tag, 0]);
	}
}

impl PartialEq for Item {
	fn eq(&self, other: &Self) -> bool {
		self.key == other.key
	}
}

impl Eq for Item {}

impl Ord for Item {
	fn cmp(&self, other: &Self) -> Ordering {
		log([b'c', self.tag, other.tag]);
		self.key.cmp(&other.key)
	}
}

impl PartialOrd for Item {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}

	fn lt(&self, other: &Self) -> bool {
		log([b'<', self.tag, other.tag]);
		self.key < other.key
	}

	fn le(&self, other: &Self) -> bool {
		log([b'l', self.tag, other.tag]);
		self.key <= other.key
	}

	fn gt(&self, other: &Self) -> bool {
		log([b'>', self.tag, other.tag]);
		self.key > other.key
	}

	fn ge(&self, other: &Self) -> bool {
		log([b'g', self.tag, other.tag]);
		self.key >= other.key
	}
}

/// Run one case.
pub fn orders(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 4 {
		0 => cmps(data, out),
		1 => sorted(data, out),
		2 => heaps(data, out),
		_ => values(data, out),
	}
	flush(out);
}

/// `cmp::max` and `min` and their forms of each byte and the next, as integers, strings,
/// and items, which the one not returned drops, and of references to items.
fn cmps(data: &[u8], out: &mut Vec<u8>) {
	for i in 1..data.len() {
		let (a, b) = (data[i - 1], data[i]);
		out.extend([cmp::max(a, b), cmp::min(a, b), cmp::max(a as i8, b as i8) as u8]);
		let (s, t) = (a.to_string(), (b / 2).to_string());
		out.extend([cmp::max(s.clone(), t.clone()).len() as u8, cmp::min(s, t).len() as u8]);
		let (x, y) = (item(2 * i, a), item(2 * i + 1, b));
		out.push(cmp::max(&x, &y).tag);
		out.push(cmp::min(&x, &y).tag);
		out.push(cmp::max(x, y).tag);
		let pair = || (item(2 * i, a), item(2 * i + 1, b));
		let (x, y) = pair();
		out.push(cmp::min(x, y).tag);
		let (x, y) = pair();
		out.push(cmp::max_by(x, y, |l, r| l.key.cmp(&r.key).reverse()).tag);
		let (x, y) = pair();
		out.push(cmp::min_by(x, y, |l, r| (l.key / 2).cmp(&(r.key / 2))).tag);
		let (x, y) = pair();
		out.push(cmp::max_by_key(x, y, |v| item(v.tag as usize + 100, v.key / 2)).tag);
		let (x, y) = pair();
		out.push(cmp::min_by_key(x, y, |v| Reverse(v.key)).tag);
		flush(out);
	}
}

/// The `is_sorted` family of slices and iterators of items, bytes, and strings, as given
/// and sorted: each test std makes, and each item or key it drops.
fn sorted(data: &[u8], out: &mut Vec<u8>) {
	let mut ascending = data.to_vec();
	ascending.sort_by_key(|b| b % 8);
	for bytes in [data, &ascending] {
		let items = || bytes.iter().enumerate().map(|(i, &b)| item(i, b));
		let v: Vec<Item> = items().collect();
		out.push(v.is_sorted() as u8);
		out.push(v.is_sorted_by(|a, b| a.key >= b.key) as u8);
		out.push(v.is_sorted_by_key(|x| item(x.tag as usize + 100, x.key / 2)) as u8);
		out.push(v.iter().is_sorted() as u8);
		out.push(v[..v.len() / 2].iter().is_sorted_by_key(|x| x.key) as u8);
		drop(v);
		flush(out);
		out.push(items().is_sorted() as u8);
		out.push(items().is_sorted_by(|a, b| a.tag > b.tag || a.key <= b.key) as u8);
		out.push(items().is_sorted_by_key(|x| Reverse(x.key)) as u8);
		out.push(items().collect::<Vec<_>>().into_iter().is_sorted() as u8);
		flush(out);
		// A function that owns an item drops as std's `all` drops it: before the last item
		// and the iterator.
		let owned = || item(200, 0);
		let (g, h, k, l) = (owned(), owned(), owned(), owned());
		out.push(items().is_sorted_by(move |a, b| (&g).key <= a.key && a.key <= b.key) as u8);
		let v = items().collect::<Vec<_>>();
		out.push(v.into_iter().is_sorted_by(move |a, b| a.key <= b.key + (&h).key) as u8);
		out.push(items().is_sorted_by_key(move |x| x.key + (&k).key) as u8);
		let key = move |x: Item| x.key + (&l).key;
		out.push(items().collect::<Vec<_>>().into_iter().is_sorted_by_key(&key) as u8);
		out.push(items().collect::<Vec<_>>().into_iter().is_sorted_by_key(Box::new(&key)) as u8);
		flush(out);
		let strings: Vec<String> = bytes.iter().map(|b| (b % 16).to_string()).collect();
		out.extend([
			bytes.is_sorted() as u8,
			bytes.is_sorted_by(|a, b| a / 4 <= b / 4) as u8,
			bytes.is_sorted_by_key(|&b| b % 8) as u8,
			bytes.iter().is_sorted() as u8,
			strings.is_sorted() as u8,
			strings.iter().map(String::len).is_sorted() as u8,
			bytes.iter().map(|&b| b as f64 / 3.0).is_sorted() as u8,
		]);
	}
}

/// A heap of items changed by each pair of bytes, an operation and its operand: each
/// comparison its sifts make, which of equal items it gives back, and when each drops.
fn heaps(data: &[u8], out: &mut Vec<u8>) {
	let mut heap = BinaryHeap::new();
	let mut other = BinaryHeap::with_capacity(4);
	let mut ops = data.iter().copied().enumerate();
	while let Some((i, op)) = ops.next() {
		let v = ops.next().map_or(0, |(_, v)| v);
		match op % 12 {
			0..=2 => heap.push(item(i, v)),
			3 => out.push(heap.pop().map_or(0xff, |x| x.tag)),
			// `DerefMut` sifts the top once it drops, but of a heap of one, and forgotten,
			// leaves the top alone in the heap.
			4 => {
				if let Some(mut top) = heap.peek_mut() {
					top.key = v % 8;
					if v & 0x10 != 0 {
						std::mem::forget(top);
					}
				}
			}
			// `Deref` alone sifts nothing, and `PeekMut::pop` pops.
			5 => {
				if let Some(top) = heap.peek_mut() {
					match v % 3 {
						0 => out.push(PeekMut::pop(top).tag),
						1 => out.push(top.tag),
						_ => {
							let mut top = top;
							top.tag ^= 0x40;
							out.push(PeekMut::pop(top).tag);
						}
					}
				}
			}
			6 => other.push(item(i, v)),
			7 => heap.append(&mut other),
			8 => heap.retain(|x| x.key != v % 8),
			9 => heap.extend(data.iter().take(v as usize % 6).map(|&b| item(i + 50, b))),
			10 => {
				let sorted = std::mem::take(&mut heap).into_sorted_vec();
				out.extend(sorted.iter().map(|x| x.tag));
				flush(out);
				heap = BinaryHeap::from(sorted);
			}
			_ => {
				out.extend(heap.iter().map(|x| x.tag));
				out.extend([
					heap.peek().map_or(0xff, |x| x.tag),
					heap.len() as u8,
					heap.is_empty() as u8,
				]);
			}
		}
		flush(out);
	}
	let mut drain = heap.drain();
	out.push(drain.next().map_or(0xff, |x| x.tag));
	drop(drain);
	flush(out);
	let mut collected: BinaryHeap<Item> = data.iter().enumerate().map(|(i, &b)| item(i, b)).collect();
	out.push(collected.pop().map_or(0xff, |x| x.tag));
	let mut rest = collected.into_iter();
	out.push(rest.next().map_or(0xff, |x| x.tag));
	drop(rest);
	flush(out);
	other.clear();
	flush(out);
}

/// Heaps of values with no comparison of their own: bytes, a min-heap of them, strings,
/// and tuples, built every way, and their vectors, slices, clones, and debug form.
fn values(data: &[u8], out: &mut Vec<u8>) {
	let mut bytes = BinaryHeap::from(data.to_vec());
	bytes.push(data.len() as u8);
	out.extend(bytes.as_slice());
	out.extend(bytes.clone().into_sorted_vec());
	out.extend(format!("{bytes:?}").bytes());
	let v: Vec<u8> = bytes.into();
	out.extend(v);
	let mut least: BinaryHeap<Reverse<u8>> = data.iter().map(|&b| Reverse(b)).collect();
	least.extend(data.iter().take(3).map(|&b| Reverse(b / 2)));
	while let Some(Reverse(b)) = least.pop() {
		out.push(b);
	}
	let strings: BinaryHeap<String> =
		BinaryHeap::from([String::from("pear"), String::from("apple"), data.len().to_string()]);
	let mut copy = strings.clone();
	copy.extend(data.iter().map(|b| (b % 10).to_string()));
	copy.retain(|s| s.len() > 1 || s.as_str() < "5");
	out.extend(copy.into_vec().iter().flat_map(|s| s.bytes()));
	out.extend(strings.into_iter().map(|s| s.len() as u8));
	let mut pairs: BinaryHeap<(u8, u8)> = BinaryHeap::new();
	pairs.extend(data.iter().map(|&b| (b % 4, b / 64)));
	pairs.extend(&[(data.len() as u8 % 4, 9)]);
	if let Some(mut top) = pairs.peek_mut() {
		top.1 = top.1.wrapping_add(1);
	}
	if let Some(mut top) = pairs.peek_mut() {
		*top = (0, 0);
	}
	out.extend(pairs.into_sorted_vec().into_iter().flat_map(|(a, b)| [a, b]));
}
