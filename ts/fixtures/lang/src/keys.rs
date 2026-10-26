//! Keys a map compares, orders, and hashes through std's impls: `Option`s, which order
//! `None` first, and `Arc`s, which compare as what they hold; lookups through `Borrow`;
//! and the references to values a map holds that JS copies, changed in place.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, btree_map, hash_map};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// A 64-bit hop, past 2^53 for some, where it is no longer a JS number.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Hop(u64);

/// A key whose derived impls read an `Option`, as moq-net's `Horizon` does.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Horizon {
	exclude: Option<Hop>,
	local: bool,
}

/// A fieldless enum, which an `Option` of compares through its own `PartialEq`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
	Left,
	Right,
}

/// Text in an `Arc`, whose derived impls compare and hash it as the text.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Name {
	text: Arc<str>,
	rank: Option<u8>,
}

/// An `Eq` that is never equal, which std's `Arc` sees past when both are one `Arc`.
struct Liar;

impl PartialEq for Liar {
	fn eq(&self, _: &Self) -> bool {
		false
	}
}

impl Eq for Liar {}

/// A `PartialEq` that is never equal, and no `Eq`, so an `Arc` of it always asks it.
struct Never;

impl PartialEq for Never {
	fn eq(&self, _: &Self) -> bool {
		false
	}
}

/// Strings whose UTF-8 order differs from their UTF-16 order past U+FFFF.
const WORDS: [&str; 6] = ["", "a", "ab", "\u{e000}", "\u{ffff}", "\u{10000}"];

fn hop(b: u8) -> Option<Hop> {
	match b % 4 {
		0 => None,
		1 => Some(Hop(b as u64 >> 2)),
		2 => Some(Hop(u64::MAX - (b >> 2) as u64)),
		_ => Some(Hop((b as u64) << 50)),
	}
}

fn small(b: u8) -> Option<u8> {
	match b % 3 {
		0 => None,
		v => Some(v),
	}
}

fn side(v: u8) -> Side {
	match v {
		1 => Side::Left,
		_ => Side::Right,
	}
}

fn name(b: u8) -> Name {
	let word = WORDS[b as usize % WORDS.len()];
	let text: Arc<str> = match b % 2 {
		0 => Arc::from(word),
		_ => Arc::from(word.to_string()),
	};
	Name {
		text,
		rank: small(b / 64),
	}
}

fn hash_of<T: Hash>(value: &T) -> u64 {
	let mut h = DefaultHasher::new();
	value.hash(&mut h);
	h.finish()
}

fn order(o: Ordering) -> u8 {
	match o {
		Ordering::Less => 1,
		Ordering::Equal => 2,
		Ordering::Greater => 3,
	}
}

/// The index of the first of `keys` that borrows as `q`.
fn find<K: Borrow<Q>, Q: PartialEq + ?Sized>(keys: &[K], q: &Q) -> u8 {
	for (i, k) in keys.iter().enumerate() {
		if k.borrow() == q {
			return i as u8;
		}
	}
	0xff
}

/// Run one case.
pub fn keys(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 3 {
		0 => options(data, out),
		1 => arcs(data, out),
		_ => places(data, out),
	}
}

/// `Option`s compared, ordered, and hashed, alone and in keys.
fn options(data: &[u8], out: &mut Vec<u8>) {
	let mut hashed: HashMap<(u8, Horizon), u8> = HashMap::new();
	let mut sorted: BTreeMap<(u8, Horizon), u8> = BTreeMap::new();
	for i in 1..data.len() {
		let (c, d) = (data[i - 1], data[i]);
		let (a, b) = (hop(c), hop(d));
		out.push((a == b) as u8 | ((a != b) as u8) << 1 | order(a.cmp(&b)) << 2);
		out.push((a.max(b) == b) as u8 | ((a.min(b) == a) as u8) << 1);
		out.push((hash_of(&a) == hash_of(&b)) as u8);
		// `Option<u8>` compares as JS's `===` does, and `Option<Side>` through `Side`'s.
		let (x, y) = (small(c), small(d));
		let (s, t) = (x.map(side), y.map(side));
		out.push((x == y) as u8 | ((s == t) as u8) << 1 | ((s.as_ref() != t.as_ref()) as u8) << 2);
		let pairs = (a.map(|h| (h.0, d as u64)), b.map(|h| (h.0, c as u64)));
		out.push((pairs.0.max(pairs.1) == pairs.1) as u8);
		let key = (
			c % 3,
			Horizon {
				exclude: a,
				local: d % 2 == 0,
			},
		);
		out.push(hashed.insert(key, i as u8).unwrap_or(0xcc));
		out.push(sorted.insert(key, i as u8).unwrap_or(0xcc));
		out.push(hashed.contains_key(&(
			d % 3,
			Horizon {
				exclude: b,
				local: false,
			},
		)) as u8);
	}
	// `None` first, then each hop in order.
	for ((n, h), v) in &sorted {
		out.push(*n);
		out.extend_from_slice(&h.exclude.map_or(0, |h| h.0).to_be_bytes());
		out.push(h.exclude.is_some() as u8 | (h.local as u8) << 1);
		out.push(*v);
		out.push(hashed[&(*n, *h)]);
	}
}

/// `Arc`s compared, ordered, and hashed as what they hold, and borrowed as it.
fn arcs(data: &[u8], out: &mut Vec<u8>) {
	for i in 1..data.len() {
		let (a, b) = (name(data[i - 1]), name(data[i]));
		out.push(order(a.text.cmp(&b.text)) | order(a.cmp(&b)) << 2);
		let max = a.text.clone().max(b.text.clone());
		out.push((a.text == b.text) as u8 | (a.eq(&b) as u8) << 1 | ((max == b.text) as u8) << 2);
		out.push((hash_of(&a.text) == hash_of(&b.text)) as u8 | ((hash_of(&a) == hash_of(&b)) as u8) << 1);
		let text: &str = a.text.borrow();
		out.push((text == b.text.as_ref()) as u8);
		let word = WORDS[data[i] as usize % WORDS.len()];
		out.push(find(&[text.to_string(), word.to_string()], word));
		out.push(find(&[a.text, b.text], word));
	}
	if let Some(&b) = data.first() {
		out.push(find(data, &b));
	}
	// A `T: Eq` is equal to itself, so std's `Arc` takes one `Arc` as equal without asking it.
	let liar = Arc::new(Liar);
	let never = Arc::new(Never);
	out.push(
		(liar == liar.clone()) as u8 | ((never == never.clone()) as u8) << 1 | ((liar == Arc::new(Liar)) as u8) << 2,
	);
}

/// Changes a map's values JS copies through the references `get_mut` and the entry API
/// give: numbers, a wide number, and a tuple, by keys JS compares and by keys found
/// through their hash.
fn places(data: &[u8], out: &mut Vec<u8>) {
	let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
	let mut wide: HashMap<u8, u64> = HashMap::new();
	let mut hops: HashMap<Hop, u16> = HashMap::new();
	let mut pairs: BTreeMap<Hop, (u8, u8)> = BTreeMap::new();
	for (i, &b) in data.iter().enumerate() {
		// Enough keys that the B-tree has more than one leaf.
		let k = b % 97;
		match b % 6 {
			0 => *counts.entry(k).or_default() += 1,
			1 => {
				if let Some(c) = counts.get_mut(&k) {
					*c -= 1;
					if *c == 0 {
						counts.remove(&k);
					}
				}
			}
			2 => match counts.entry(k) {
				btree_map::Entry::Occupied(mut e) if *e.get() > 1 => *e.get_mut() *= 3,
				btree_map::Entry::Occupied(e) => *e.into_mut() += 7,
				btree_map::Entry::Vacant(e) => *e.insert(i) += 2,
			},
			3 => {
				let c = counts.entry(k).or_insert_with(|| i * 2);
				*c ^= 5;
				out.push(*c as u8);
			}
			4 if counts.contains_key(&k) => *counts.get_mut(&k).expect("a counted key") += 4,
			4 => *counts.entry(k).or_insert(9) <<= 1,
			// A missing key panics.
			_ => *counts.get_mut(&k).unwrap() += 1,
		}
		let w = wide.entry(b % 7).or_insert(1 << 52);
		*w = w.wrapping_add(*w >> 1).wrapping_add(b as u64);
		let h = hops.entry(Hop(b as u64 % 6)).or_default();
		*h = h.wrapping_add(b as u16);
		if let Some(h) = hops.get_mut(&Hop(b as u64 % 4)) {
			*h ^= 0x5a5a;
		}
		match hops.entry(Hop(b as u64 % 5)) {
			hash_map::Entry::Occupied(mut e) if *e.get() > 0x1000 => *e.get_mut() -= 0x1000,
			hash_map::Entry::Occupied(e) => *e.into_mut() |= 1,
			hash_map::Entry::Vacant(e) => *e.insert(b as u16) <<= 4,
		}
		let p = pairs.entry(Hop(b as u64 % 3)).or_insert((0, 0));
		p.0 = p.0.wrapping_add(b);
		p.1 ^= p.0;
	}
	// A reference into a map, written by a closure, swapped whole, and moved.
	let c = counts.entry(0).or_default();
	let mut bump = || *c += 1;
	bump();
	let mut spare = [data.len(), 8];
	let c = counts.entry(1).or_insert(3);
	std::mem::swap(c, &mut spare[0]);
	let d = counts.entry(2).or_default();
	let e = d;
	*e += spare[0];
	for (k, c) in &counts {
		out.push(*k);
		out.extend_from_slice(&(*c as u32).to_be_bytes());
	}
	for k in 0..7 {
		out.extend_from_slice(&wide.get(&k).copied().unwrap_or(0).to_be_bytes());
	}
	for k in 0..6 {
		out.extend_from_slice(&hops.get(&Hop(k)).copied().unwrap_or(0).to_be_bytes());
	}
	for (k, (a, b)) in &pairs {
		out.push(k.0 as u8);
		out.push(*a);
		out.push(*b);
	}
}
