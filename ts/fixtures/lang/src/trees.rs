//! Whole B-trees of values that log as they drop, which std drops node by node, each
//! node's entries in order after its children's: maps built by each insertion order, by
//! `collect`, emptied in part by removals that merge nodes, cloned, split, and nested, sets,
//! and `IntoIter`, which drops what it still holds in key order.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

thread_local! {
	static LOG: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Puts what was logged since the last flush.
fn flush(out: &mut Vec<u8>) {
	LOG.with_borrow_mut(|log| {
		out.extend_from_slice(log);
		log.clear();
	});
}

/// Logs its tag as it drops, and orders by it as a set's key.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Item(u16);

impl Drop for Item {
	fn drop(&mut self) {
		LOG.with_borrow_mut(|log| log.extend(self.0.to_le_bytes()));
	}
}

/// A clone, told apart from the original by its tag.
impl Clone for Item {
	fn clone(&self) -> Self {
		Item(self.0 + 1000)
	}
}

/// xorshift32, seeded by the input, so a short input scripts a tree of hundreds of entries.
struct Rand(u32);

impl Rand {
	fn next(&mut self) -> u32 {
		self.0 ^= self.0 << 13;
		self.0 ^= self.0 >> 17;
		self.0 ^= self.0 << 5;
		self.0
	}
}

/// `kind` picks how the tree is built and dropped. The first byte of `data` sets how many
/// entries, up to 300 so trees reach height 2, the next ones seed its keys, and the bit
/// of each key a removal script takes.
pub fn trees(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	// What a case that panicked left, which a translation outlives, as native Rust's trap does not.
	LOG.with_borrow_mut(Vec::clear);
	let &[count, ref seed @ ..] = data else {
		return;
	};
	let n = count as u16 + count as u16 / 6;
	let mut rand = Rand(seed.iter().fold(0x9e37_79b9u32, |s, &b| (s << 5 | s >> 27) ^ b as u32) | 1);
	// Keys apart enough that a random script repeats some.
	let mut key = || (rand.next() % 400) as u16;
	match kind % 10 {
		// By random inserts, whose replaced values drop as they go.
		0 => {
			let mut map = BTreeMap::new();
			for _ in 0..n {
				let k = key();
				map.insert(k, Item(k));
			}
			flush(out);
			out.push(0xaa);
			drop(map);
		}
		// In ascending order, then descending, which leave nodes of other sizes.
		1 => {
			let mut map = BTreeMap::new();
			for k in 0..n {
				map.insert(k, Item(k));
			}
			drop(map);
			out.push(0xaa);
			let mut map = BTreeMap::new();
			for k in (0..n).rev() {
				map.insert(k, Item(k));
			}
			drop(map);
		}
		// By `collect`, std's bulk build from sorted entries.
		2 => {
			let map: BTreeMap<u16, Item> = (0..n).map(|_| key()).map(|k| (k, Item(k))).collect();
			flush(out);
			out.push(0xaa);
			drop(map);
		}
		// Removals after random inserts, which steal from and merge nodes, then pops.
		3 => {
			let mut map = BTreeMap::new();
			for _ in 0..n {
				let k = key();
				map.insert(k, Item(k));
			}
			for _ in 0..n {
				let k = key();
				if k % 3 != 0 {
					out.extend(map.remove(&k).map(|item| item.0 as u8));
				}
			}
			for _ in 0..n / 8 {
				out.extend(map.pop_first().map(|(k, _)| k as u8));
				out.extend(map.pop_last().map(|(k, _)| k as u8));
			}
			flush(out);
			out.push(0xaa);
			drop(map);
		}
		// A set, by inserts and removals.
		4 => {
			let mut set = BTreeSet::new();
			for _ in 0..n {
				set.insert(Item(key()));
			}
			for _ in 0..n / 2 {
				set.remove(&Item(key()));
			}
			flush(out);
			out.push(0xaa);
			drop(set);
		}
		// Maps of maps, each of which drops its own nodes as its outer entry drops.
		5 => {
			let mut map: BTreeMap<u16, BTreeMap<u16, Item>> = BTreeMap::new();
			for _ in 0..n {
				let (k, j) = (key() % 24, key());
				map.entry(k).or_default().insert(j, Item(j));
			}
			flush(out);
			out.push(0xaa);
			drop(map);
		}
		// `IntoIter`, read from both ends, drops what it still holds in key order.
		6 => {
			let mut map = BTreeMap::new();
			for _ in 0..n {
				let k = key();
				map.insert(k, Item(k));
			}
			flush(out);
			let mut iter = map.into_iter();
			for _ in 0..seed.len() {
				out.extend(iter.next().map(|(k, _)| k as u8));
				out.extend(iter.next_back().map(|(k, _)| k as u8));
			}
			flush(out);
			out.push(0xaa);
			drop(iter);
		}
		// `clear`, and a map assigned over.
		7 => {
			let mut map = BTreeMap::new();
			for _ in 0..n {
				let k = key();
				map.insert(k, Item(k));
			}
			flush(out);
			map.clear();
			flush(out);
			out.push(0xaa);
			for k in 0..n {
				map.insert(k, Item(k));
			}
			map = BTreeMap::new();
			flush(out);
			out.push(map.len() as u8);
		}
		// `retain`, and a map split in two, each of which drops as its nodes are.
		8 => {
			let mut map = BTreeMap::new();
			for _ in 0..n {
				let k = key();
				map.insert(k, Item(k));
			}
			map.retain(|k, _| k % 5 != 0);
			flush(out);
			out.push(0xaa);
			let high = map.split_off(&200);
			drop(map);
			out.push(0xbb);
			flush(out);
			drop(high);
		}
		// A clone, which std builds node for node as the original.
		_ => {
			let mut map = BTreeMap::new();
			for _ in 0..n {
				let k = key();
				map.insert(k, Item(k));
			}
			for _ in 0..n / 2 {
				map.remove(&key());
			}
			flush(out);
			let copy = map.clone();
			drop(map);
			out.push(0xaa);
			flush(out);
			drop(copy);
		}
	}
	flush(out);
}
