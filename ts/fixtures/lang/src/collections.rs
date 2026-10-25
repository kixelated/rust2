//! Collections, each driven by a script of operations read from the input: keys JS
//! compares as Rust does, 64-bit keys past 2^53, strings, and structs, tuples, and enums
//! found through their translated `Hash` and `Eq` or ordered by their `Ord`. A hash
//! map's iteration order is unspecified, so only what does not depend on it is written.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque, btree_map, hash_map};
use std::hash::{DefaultHasher, Hash, Hasher};

/// A key whose `Eq`, `Hash`, and `Ord` see only its low nibble, so two keys can be equal
/// while their bytes differ, and which sorts by that nibble, descending.
#[derive(Clone, Copy)]
struct Nibble(u8);

impl PartialEq for Nibble {
	fn eq(&self, other: &Self) -> bool {
		self.0 & 0xf == other.0 & 0xf
	}
}

impl Eq for Nibble {}

impl Hash for Nibble {
	fn hash<H: Hasher>(&self, state: &mut H) {
		(self.0 & 0xf).hash(state);
	}
}

impl Ord for Nibble {
	fn cmp(&self, other: &Self) -> Ordering {
		(other.0 & 0xf).cmp(&(self.0 & 0xf))
	}
}

impl PartialOrd for Nibble {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Pair {
	a: u8,
	b: u16,
}

/// A value changed in place through `get_mut` and `entry`: a JS number can't be.
#[derive(Clone, Copy, Default)]
struct Tally {
	n: u16,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Rank {
	Low,
	Mid(u8),
	High { x: u8, y: u8 },
}

/// A key whose `Hash` writes nothing, so every key collides.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Clash(u8);

impl Hash for Clash {
	fn hash<H: Hasher>(&self, _: &mut H) {}
}

/// A fieldless enum whose own `Eq` and `Hash` find `Up` and `Top` equal, which their JS
/// tags are not.
#[derive(Clone, Copy)]
enum Dir {
	Up,
	Top,
	Down,
}

impl Dir {
	fn rising(self) -> bool {
		!matches!(self, Dir::Down)
	}
}

impl PartialEq for Dir {
	fn eq(&self, other: &Self) -> bool {
		self.rising() == other.rising()
	}
}

impl Eq for Dir {}

impl Hash for Dir {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.rising().hash(state);
	}
}

/// Strings whose UTF-8 order differs from their UTF-16 order: past U+FFFF, a surrogate
/// pair sorts below U+E000 in UTF-16, and above it in UTF-8, as Rust sorts.
const WORDS: [&str; 8] = ["", "a", "ab", "b", "\u{e9}", "\u{e000}", "\u{ffff}", "\u{10000}"];

/// A 64-bit key: small, or past 2^53, where it is no longer a JS number.
fn wide(b: u8) -> u64 {
	match b % 3 {
		0 => b as u64,
		1 => (b as u64) << 50,
		_ => u64::MAX - b as u64,
	}
}

fn rank(b: u8, c: u8) -> Rank {
	match b % 3 {
		0 => Rank::Low,
		1 => Rank::Mid(c % 4),
		_ => Rank::High { x: c % 2, y: c % 3 },
	}
}

fn rank_byte(r: &Rank) -> u8 {
	match *r {
		Rank::Low => 0,
		Rank::Mid(m) => 0x10 | m,
		Rank::High { x, y } => 0x20 | x << 2 | y,
	}
}

/// A `HashMap` by each kind of key, through a script of inserts, lookups, and removals.
pub fn hash_maps(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 4 {
		0 => {
			let mut map: HashMap<u8, u16> = HashMap::new();
			for i in (0..data.len()).step_by(2) {
				let (k, v) = (data[i] % 16, data.get(i + 1).copied().unwrap_or(0) as u16);
				match v % 5 {
					0 => out.push(map.remove(&k).map_or(0xee, |old| old as u8)),
					1 => out.push(map.contains_key(&k) as u8),
					2 => out.push(map.get(&k).map_or(0xdd, |v| *v as u8)),
					_ => out.push(map.insert(k, v * 3).map_or(0xcc, |old| old as u8)),
				}
			}
			out.push(map.len() as u8);
			out.push(map.is_empty() as u8);
			let total: u16 = map.values().map(|v| v % 256).sum();
			out.extend_from_slice(&total.to_be_bytes());
			let sorted: BTreeMap<u8, u16> = map.iter().map(|(k, v)| (*k, *v)).collect();
			for (k, v) in &sorted {
				out.push(*k);
				out.extend_from_slice(&v.to_be_bytes());
			}
			map.clear();
			out.push(map.len() as u8);
		}
		1 => {
			let mut map: HashMap<u64, u8> = HashMap::with_capacity(4);
			for (i, &b) in data.iter().enumerate() {
				let k = wide(b % 12);
				match b / 12 % 4 {
					0 => out.push(map.remove(&k).unwrap_or(0xee)),
					1 => out.push(map.get(&k).copied().unwrap_or(0xdd)),
					_ => out.push(map.insert(k, i as u8).unwrap_or(0xcc)),
				}
			}
			let keys: BTreeSet<u64> = map.keys().copied().collect();
			for k in &keys {
				out.extend_from_slice(&k.to_be_bytes());
				out.push(map[k]);
			}
			// A missing key panics.
			if let Some(&b) = data.first() {
				out.push(map[&wide(b % 13)]);
			}
		}
		2 => {
			// The key an insert keeps is the first equal one: Rust replaces only the value.
			let mut map: HashMap<Nibble, u8> = HashMap::new();
			let mut pairs: HashMap<Pair, Tally> = HashMap::new();
			let mut ranks: HashMap<Rank, Tally> = HashMap::new();
			let mut tuples: HashMap<(u8, &str), u8> = HashMap::new();
			for i in 1..data.len() {
				let (a, b) = (data[i - 1], data[i]);
				out.push(map.insert(Nibble(a), b).unwrap_or(0xcc));
				let pair = Pair {
					a: a % 4,
					b: b as u16 % 3,
				};
				let t = pairs.entry(pair).or_default();
				t.n += 1;
				let r = rank(a, b);
				if let Some(t) = ranks.get_mut(&r) {
					t.n += 1;
				} else {
					ranks.insert(r, Tally { n: i as u16 });
				}
				let t = (a % 3, WORDS[b as usize % WORDS.len()]);
				out.push(tuples.remove(&t).unwrap_or(0xbb));
				if a % 2 == 0 {
					tuples.insert(t, a);
				}
			}
			let raw: BTreeMap<u8, u8> = map.iter().map(|(k, v)| (k.0, *v)).collect();
			for (k, v) in raw {
				out.push(k);
				out.push(v);
			}
			// A stored value is its own copy, as is one read back out.
			let key = Pair { a: 9, b: 9 };
			let mut local = Tally { n: 7 };
			pairs.insert(key, local);
			local.n = 8;
			let before = pairs[&key];
			if let Some(t) = pairs.get_mut(&key) {
				t.n += 2;
			}
			out.push(local.n as u8);
			out.push(before.n as u8);
			out.push(pairs[&key].n as u8);
			let pairs: BTreeMap<Pair, Tally> = pairs.into_iter().collect();
			for (p, t) in pairs {
				out.push(p.a);
				out.push(p.b as u8);
				out.push(t.n as u8);
			}
			// The entry API, and a clone whose values change apart from its source's.
			for i in 1..data.len() {
				let (a, b) = (data[i - 1], data[i]);
				let t = ranks
					.entry(rank(b, a))
					.and_modify(|t| t.n ^= 3)
					.or_insert_with(|| Tally { n: b as u16 });
				t.n = t.n.wrapping_add(t.n);
				let e = ranks.entry(rank(data[i], 0));
				out.push(rank_byte(e.key()));
				out.push(e.or_insert_with_key(|k| Tally { n: rank_byte(k) as u16 }).n as u8);
			}
			let mut copy = ranks.clone();
			if let Some(t) = copy.get_mut(&Rank::Low) {
				t.n = 0x77;
			}
			out.push(ranks.get(&Rank::Low).map_or(0xff, |t| t.n as u8));
			out.push(copy.get(&Rank::Low).map_or(0xff, |t| t.n as u8));
			out.push(
				copy.remove_entry(&Rank::Mid(1))
					.map_or(0xff, |(k, t)| rank_byte(&k) ^ t.n as u8),
			);
			let bytes: HashMap<u8, Tally> = data.iter().map(|&b| (b % 8, Tally { n: b as u16 })).collect();
			let mut copy = bytes.clone();
			for t in copy.values_mut() {
				t.n += 1;
			}
			for k in 0..8 {
				out.push(bytes.get(&k).map_or(0xff, |t| t.n as u8));
				out.push(copy.get(&k).map_or(0xff, |t| t.n as u8));
			}
			for &b in data {
				out.push(claim(&mut copy, b % 8, b as u16));
			}
			// Extending by reference copies each value.
			let mut extended: HashMap<u8, Tally> = HashMap::new();
			extended.extend(&bytes);
			for t in extended.values_mut() {
				t.n ^= 0x11;
			}
			for k in 0..8 {
				out.push(bytes.get(&k).map_or(0xff, |t| t.n as u8));
				out.push(extended.get(&k).map_or(0xff, |t| t.n as u8));
			}
			// Equal keys hash equal through `DefaultHasher`, whose values are otherwise unspecified.
			let hash = |n: Nibble| {
				let mut h = DefaultHasher::new();
				n.hash(&mut h);
				h.finish()
			};
			let first = data.first().copied().unwrap_or(0);
			out.push((hash(Nibble(first)) == hash(Nibble(first ^ 0xf0))) as u8);
			ranks.retain(|r, t| t.n % 2 == 0 || *r == Rank::Low);
			let ranks: BTreeMap<Rank, u16> = ranks.iter().map(|(r, t)| (*r, t.n)).collect();
			for (r, n) in &ranks {
				out.push(rank_byte(r));
				out.push(*n as u8);
			}
			out.push(tuples.len() as u8);
		}
		_ => {
			let mut words: HashMap<String, Vec<u8>> = HashMap::new();
			for &b in data {
				let word = WORDS[b as usize % WORDS.len()].to_string();
				words.entry(word).or_default().push(b);
			}
			let mut lens: BTreeMap<String, usize> = BTreeMap::new();
			for (w, bytes) in &words {
				lens.insert(w.clone(), bytes.len());
			}
			for (w, n) in &lens {
				out.extend_from_slice(w.as_bytes());
				out.push(*n as u8);
			}
			let drained: usize = words.drain().map(|(_, v)| v.len()).sum();
			out.push(drained as u8);
			out.push(words.is_empty() as u8);
			// Every key hashes alike, so each lookup walks the keys that collide.
			let mut clashes: HashMap<Clash, Tally> = HashMap::new();
			for (i, &b) in data.iter().enumerate() {
				let k = Clash(b % 8);
				match b / 8 % 4 {
					0 => out.push(clashes.remove(&k).map_or(0xee, |t| t.n as u8)),
					1 => out.push(clashes.get(&k).map_or(0xdd, |t| t.n as u8)),
					_ => out.push(clashes.insert(k, Tally { n: i as u16 }).map_or(0xcc, |t| t.n as u8)),
				}
			}
			clashes.retain(|k, t| k.0 % 3 != 0 || t.n % 2 == 0);
			let kept: BTreeSet<u8> = clashes.keys().map(|k| k.0).collect();
			out.extend(kept.iter());
			let copy = clashes.clone();
			out.push(copy.len() as u8);
			out.push(copy.contains_key(&Clash(1)) as u8);
		}
	}
}

/// Matching on an entry: add to an even tally, take out an odd one, or start one.
fn claim(map: &mut HashMap<u8, Tally>, k: u8, n: u16) -> u8 {
	match map.entry(k) {
		hash_map::Entry::Occupied(mut e) if e.get().n % 2 == 0 => {
			let t = e.get_mut();
			t.n += n;
			1
		}
		hash_map::Entry::Occupied(e) => e.remove().n as u8,
		hash_map::Entry::Vacant(e) => {
			e.insert(Tally { n });
			0
		}
	}
}

/// `HashSet` and `BTreeSet`, each with the same script.
pub fn sets(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 3 {
		0 => {
			let mut hashed: HashSet<u16> = HashSet::new();
			let mut sorted: BTreeSet<u16> = BTreeSet::new();
			for &b in data {
				let v = b as u16 % 24;
				match b % 3 {
					0 => {
						out.push(hashed.remove(&v) as u8);
						out.push(sorted.remove(&v) as u8);
					}
					1 => {
						out.push(hashed.contains(&v) as u8);
						out.push(sorted.contains(&v) as u8);
					}
					_ => {
						out.push(hashed.insert(v) as u8);
						out.push(sorted.insert(v) as u8);
					}
				}
			}
			out.push(hashed.len() as u8);
			out.push(hashed.iter().filter(|v| **v % 2 == 1).count() as u8);
			for v in sorted.iter().rev() {
				out.push(*v as u8);
			}
			out.push(sorted.first().copied().unwrap_or(0xff) as u8);
			out.push(sorted.last().copied().unwrap_or(0xff) as u8);
			let firsts: Vec<u16> = sorted.range(4..12).copied().collect();
			out.push(firsts.len() as u8);
			hashed.retain(|v| v % 3 != 0);
			sorted.retain(|v| v % 3 != 0);
			out.push(hashed.len() as u8);
			out.extend(sorted.iter().map(|v| *v as u8));
		}
		1 => {
			let mut nibbles: BTreeSet<Nibble> = BTreeSet::new();
			let mut seen: HashSet<Nibble> = HashSet::new();
			for &b in data {
				out.push(nibbles.insert(Nibble(b)) as u8);
				out.push(seen.insert(Nibble(b)) as u8);
			}
			for n in &nibbles {
				out.push(n.0);
			}
			while let Some(n) = nibbles.pop_last() {
				out.push(n.0);
				if nibbles.len() % 2 == 0 {
					out.push(nibbles.pop_first().map_or(0xee, |n| n.0));
				}
			}
			out.push(seen.len() as u8);
			let mut dirs: HashSet<Dir> = HashSet::new();
			for &b in data {
				let dir = [Dir::Up, Dir::Top, Dir::Down][b as usize % 3];
				out.push(dirs.insert(dir) as u8);
			}
			out.push(dirs.len() as u8);
		}
		_ => {
			let mut words: BTreeSet<&str> = BTreeSet::new();
			for &b in data {
				words.insert(WORDS[b as usize % WORDS.len()]);
			}
			for w in &words {
				out.extend_from_slice(w.as_bytes());
				out.push(0);
			}
			let wides: BTreeSet<u64> = data.iter().map(|&b| wide(b)).collect();
			for w in wides.iter().rev() {
				out.extend_from_slice(&w.to_be_bytes());
			}
		}
	}
}

/// `BTreeMap`: ordered by the key's `Ord`, through inserts, removals, and ranges.
pub fn btree_maps(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 3 {
		0 => {
			let mut map: BTreeMap<u8, u8> = BTreeMap::new();
			for i in (0..data.len()).step_by(2) {
				let (k, v) = (data[i] % 32, data.get(i + 1).copied().unwrap_or(0));
				match v % 6 {
					0 => out.push(map.remove(&k).unwrap_or(0xee)),
					1 => out.push(map.pop_first().map_or(0xdd, |(k, v)| k ^ v)),
					2 => out.push(map.pop_last().map_or(0xdc, |(k, v)| k ^ v)),
					3 => {
						if let Some(v) = map.get(&k).copied() {
							map.insert(k, v.wrapping_add(1));
						}
						out.push(map.get(&k).copied().unwrap_or(0xdb));
					}
					_ => out.push(map.insert(k, v).unwrap_or(0xcc)),
				}
			}
			out.push(map.first_key_value().map_or(0xff, |(k, _)| *k));
			out.push(map.last_key_value().map_or(0xff, |(k, _)| *k));
			for (k, v) in map.range(8..24) {
				out.push(*k);
				out.push(*v);
			}
			for (k, v) in map.range(10..).rev() {
				out.push(k ^ v);
			}
			for v in map.range(..6).map(|(_, v)| *v) {
				out.push(v);
			}
			out.extend(map.keys().copied());
			out.extend(map.values().copied());
			map.remove(&3);
			out.push(map.len() as u8);
			out.push(map.contains_key(&3) as u8);
			// A range whose start is past its end panics.
			let lo = data.first().copied().unwrap_or(0) % 32;
			let hi = data.get(1).copied().unwrap_or(0) % 32;
			let within: Vec<u8> = match lo % 3 {
				0 => map.range(lo..hi).map(|(k, _)| *k).collect(),
				1 => map.range(lo..=hi).map(|(k, _)| *k).collect(),
				_ => map.range(lo..).map(|(k, _)| *k).collect(),
			};
			out.extend_from_slice(&within);
			out.extend(map.into_values());
		}
		1 => {
			let mut ranks: BTreeMap<Rank, Tally> = BTreeMap::new();
			let mut pairs: BTreeMap<(u8, u8), u8> = BTreeMap::new();
			let mut wides: BTreeMap<u64, u8> = BTreeMap::new();
			for i in 1..data.len() {
				let (a, b) = (data[i - 1], data[i]);
				let t = ranks.entry(rank(a, b)).or_default();
				t.n = t.n.wrapping_add(b as u16);
				pairs.insert((a % 4, b % 4), a);
				wides.entry(wide(a)).or_insert(b);
			}
			if let Some(t) = ranks.get_mut(&Rank::Low) {
				t.n ^= 0x5a5a;
			}
			for &b in data {
				let r = rank(b, b / 3);
				match ranks.entry(r) {
					btree_map::Entry::Vacant(e) => {
						out.push(rank_byte(e.key()));
						e.insert(Tally { n: b as u16 });
					}
					btree_map::Entry::Occupied(e) if e.get().n > 0x100 => out.push(rank_byte(&e.remove_entry().0)),
					btree_map::Entry::Occupied(mut e) => out.push(e.insert(Tally { n: 0x200 }).n as u8),
				}
			}
			for (r, t) in &ranks {
				out.push(rank_byte(r));
				out.extend_from_slice(&t.n.to_be_bytes());
			}
			for ((a, b), v) in &pairs {
				out.push(a << 4 | b);
				out.push(*v);
			}
			for (k, v) in wides.range(1u64 << 52..) {
				out.extend_from_slice(&k.to_be_bytes());
				out.push(*v);
			}
		}
		_ => {
			let mut nibbles: BTreeMap<Nibble, u8> = BTreeMap::new();
			let mut words: BTreeMap<&str, Tally> = BTreeMap::new();
			for &b in data {
				out.push(nibbles.insert(Nibble(b), b).unwrap_or(0xcc));
				let t = words.entry(WORDS[b as usize % WORDS.len()]).or_default();
				t.n += 1;
			}
			for (k, v) in &nibbles {
				out.push(k.0);
				out.push(*v);
			}
			// `Ordering`'s methods, on an `Ord` that sorts descending.
			for i in 1..data.len() {
				let (a, b) = (Nibble(data[i - 1]), Nibble(data[i]));
				let (x, y) = (data[i - 1], data[i]);
				let o = a.cmp(&b).reverse().then_with(|| y.cmp(&x));
				out.push(
					o.is_lt() as u8 | (o.is_ge() as u8) << 1 | (a.cmp(&b).then(Ordering::Less).is_eq() as u8) << 2,
				);
				out.push(match o {
					Ordering::Less => 1,
					Ordering::Equal => 2,
					Ordering::Greater => 3,
				});
				out.push((o == Ordering::Greater) as u8);
			}
			// `collect` keeps the last of equal keys, and `extend` the first.
			let collected: BTreeMap<Nibble, u8> = data.iter().map(|&b| (Nibble(b), b)).collect();
			let mut keys: BTreeSet<Nibble> = data.iter().map(|&b| Nibble(b)).collect();
			keys.extend(data.iter().map(|&b| Nibble(b ^ 0x30)));
			for ((k, v), n) in collected.iter().zip(keys.iter()) {
				out.push(k.0);
				out.push(*v);
				out.push(n.0);
			}
			words.retain(|w, t| w.len() > 1 || t.n > 1);
			for (w, t) in &words {
				out.extend_from_slice(w.as_bytes());
				out.push(t.n as u8);
			}
		}
	}
}

/// `VecDeque`: a ring buffer, pushed and popped at both ends past its capacity.
pub fn deques(data: &[u8], out: &mut Vec<u8>) {
	let mut q: VecDeque<u8> = VecDeque::new();
	let mut pairs: VecDeque<(u8, u8)> = VecDeque::with_capacity(2);
	for &b in data {
		match b % 7 {
			0 => out.push(q.pop_front().unwrap_or(0xee)),
			1 => out.push(q.pop_back().unwrap_or(0xed)),
			2 => q.push_front(b),
			3 => {
				out.push(q.front().copied().unwrap_or(0xec));
				out.push(q.back().copied().unwrap_or(0xeb));
				out.push(q.get(b as usize % 5).copied().unwrap_or(0xea));
			}
			_ => q.push_back(b),
		}
		pairs.push_back((b, q.len() as u8));
		if pairs.len() > 3 {
			pairs.pop_front();
		}
	}
	out.push(q.len() as u8);
	out.extend(q.iter().rev());
	if !q.is_empty() {
		out.push(q[q.len() / 2]);
	}
	q.retain(|b| b % 2 == 0);
	out.extend(q.iter());
	for (a, b) in &pairs {
		out.push(a ^ b);
	}
	let drained: Vec<u8> = q.drain(..).collect();
	out.extend_from_slice(&drained);
	out.push(q.is_empty() as u8);
	// A clone's items change apart from its source's.
	let tallies: VecDeque<Tally> = data.iter().map(|&b| Tally { n: b as u16 }).collect();
	let mut copy = tallies.clone();
	if let Some(t) = copy.front_mut() {
		t.n ^= 0xff;
	}
	out.extend(tallies.iter().map(|t| t.n as u8));
	out.extend(copy.iter().map(|t| t.n as u8));
	copy.extend(tallies.iter());
	if let Some(t) = copy.back_mut() {
		t.n ^= 0xff;
	}
	out.extend(tallies.iter().map(|t| t.n as u8));
	out.extend(copy.iter().map(|t| t.n as u8));
	q.extend(data.iter().copied());
	q.insert(q.len() / 2, 0x42);
	out.push(q.remove(1).unwrap_or(0xe9));
	q.truncate(5);
	out.push(q.contains(&0x42) as u8);
	out.extend(q.iter().rev());
	// Draining or indexing past the end panics.
	let a = data.first().copied().unwrap_or(0) as usize % 8;
	let b = data.get(1).copied().unwrap_or(0) as usize % 8;
	let mut q: VecDeque<u8> = data.iter().copied().collect();
	let drained: Vec<u8> = match a % 4 {
		0 => q.drain(a..b).collect(),
		1 => q.drain(a..=b).collect(),
		2 => q.drain(..b).collect(),
		_ => q.drain(a..).collect(),
	};
	out.extend_from_slice(&drained);
	out.extend(q.iter());
	out.push(q[b]);
}
