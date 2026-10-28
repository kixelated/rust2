//! Collections, each driven by a script of operations read from the input: keys JS
//! compares as Rust does, 64-bit keys past 2^53, strings, and structs, tuples, and enums
//! found through their translated `Hash` and `Eq` or ordered by their `Ord`. A hash
//! map's iteration order is unspecified, so only what does not depend on it is written.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque, btree_map, hash_map};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::generics::Wrap;
use crate::iters::Bumped;

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

/// A value whose `clone` panics on a multiple of 7, saying which.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Fussy(u8);

impl Clone for Fussy {
	fn clone(&self) -> Self {
		if self.0 % 7 == 0 {
			panic!("cloning {}", self.0);
		}
		Fussy(self.0)
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
			let mut it = map.iter();
			out.push(it.by_ref().count() as u8);
			out.push(it.next().is_none() as u8);
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
			// A capacity call still evaluates its receiver.
			let mut stamped: HashMap<u8, Tally> = HashMap::new();
			stamp(&mut stamped).shrink_to_fit();
			stamp(&mut stamped).reserve(2);
			out.push(stamped.len() as u8);
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
			// A hasher found by a call evaluates before the value written to it.
			let (mut h, mut counted) = (DefaultHasher::new(), Tally { n: first as u16 });
			counted_hasher(&mut h, &mut counted).write_u16(counted.n);
			let mut want = DefaultHasher::new();
			want.write_u16(first as u16 + 1);
			out.push((h.finish() == want.finish()) as u8);
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
/// Counts a write, and gives back the hasher.
fn counted_hasher<'a>(h: &'a mut DefaultHasher, counted: &mut Tally) -> &'a mut DefaultHasher {
	counted.n += 1;
	h
}

/// Inserts a key, and gives back the map.
fn stamp(map: &mut HashMap<u8, Tally>) -> &mut HashMap<u8, Tally> {
	map.insert(map.len() as u8, Tally::default());
	map
}

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
	match kind % 4 {
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
		// `clone` clones each key and then its value, so which `Fussy` panics first is std's.
		2 => {
			let map: BTreeMap<Fussy, Fussy> = data.iter().map(|&b| (Fussy(b), Fussy(b ^ 0x5a))).collect();
			let copy = map.clone();
			for (k, v) in &copy {
				out.push(k.0 ^ v.0);
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

/// Pushes an item, and gives back the deque.
fn touch(q: &mut VecDeque<u8>) -> &mut VecDeque<u8> {
	q.push_back(0x42);
	q
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
	touch(&mut q).shrink_to_fit();
	touch(&mut q).reserve(1);
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

/// A value that counts how far it is cloned from the one `vec!` was given, which
/// `vec![x; n]` clones for every element but the last, which is `x` itself.
struct Cloned {
	depth: u8,
	name: String,
}

impl Clone for Cloned {
	fn clone(&self) -> Self {
		Cloned {
			depth: self.depth + 1,
			name: self.name.clone(),
		}
	}
}

/// `vec![a, b]` and `vec![x; n]`, of bytes, numbers, `Copy` tuples, and types that clone.
pub fn vecs(data: &[u8], out: &mut Vec<u8>) {
	let a = data.first().copied().unwrap_or(0);
	let b = data.get(1).copied().unwrap_or(0);
	let n = data.get(2).copied().unwrap_or(0) as usize % 5;
	let name = match String::from_utf8(data.to_vec()) {
		Ok(s) => s,
		Err(_) => "?".to_string(),
	};

	let bytes = vec![a, b, 0x42];
	out.extend_from_slice(&bytes);
	let mut pairs = vec![(a, b as u16)];
	pairs.push((b, a as u16));
	pairs[0].0 ^= 0xff;
	for (x, y) in &pairs {
		out.push(x ^ *y as u8);
	}
	let names = vec![name.clone(), "x".to_string(), name.clone()];
	out.push(names.len() as u8);
	let one = vec![Cloned {
		depth: a,
		name: name.clone(),
	}];
	out.push(one[0].depth);

	out.extend_from_slice(&vec![b; n]);
	let wide = vec![a as u16 * 0x101; n];
	out.extend(wide.iter().map(|w| (w >> 4) as u8));
	let mut tuples = vec![(a, b); n];
	if n > 0 {
		tuples[0].0 = 0xee;
	}
	out.extend(tuples.iter().map(|t| t.0 ^ t.1));
	let strings = vec![name.clone(); n];
	out.push(strings.len() as u8);
	let cloned = vec![Cloned { depth: 0, name }; n];
	out.extend(cloned.iter().map(|c| c.depth));
	out.push(cloned.iter().all(|c| c.name.len() == data.len()) as u8);
	// A `Copy` type's own `Clone` runs too.
	let bumped = vec![Bumped(b); n];
	out.extend(bumped.iter().map(|o| o.0));

	// Each element is a copy of what it was given, apart from it and from each other.
	let pair = (a, b);
	let held = Bumped(a);
	let mut listed = vec![pair, pair];
	listed[0].0 ^= 0xff;
	let mut repeated = vec![pair; n];
	let again = vec![pair; n];
	let mut bumps = vec![held; n];
	let kept = vec![held; n];
	if n > 0 {
		repeated[n - 1].1 ^= 0xff;
		bumps[n - 1].0 ^= 0xff;
	}
	out.extend_from_slice(&[pair.0, pair.1, listed[1].0, held.0]);
	out.extend(repeated.iter().chain(&again).map(|t| t.1));
	out.extend(bumps.iter().chain(&kept).map(|o| o.0));

	// A `Vec`'s slice is bytes only when its own elements are bytes.
	let mut grown = bytes.clone();
	grown.as_mut_slice()[0] ^= 0xff;
	out.extend_from_slice(grown.as_slice());
	let nested = vec![bytes, vec![b; n]];
	for v in nested.as_slice() {
		out.push(v.len() as u8);
		out.extend_from_slice(v);
	}
	let mut wrapped = vec![Wrap { value: a, count: b }; n];
	if n > 0 {
		wrapped.as_mut_slice()[n - 1].count ^= 0xff;
	}
	out.extend(wrapped.as_slice().iter().map(|w| w.value ^ w.count));
	out.extend(names.as_slice().iter().map(|s| s.len() as u8));
}

/// A struct that is not `Copy` holding a `Copy` aggregate by value.
struct Holder {
	pair: (u8, u8),
}

/// Stores `p` by value in `v`.
fn keep(v: &mut Vec<(u8, u8)>, p: (u8, u8)) {
	v.push(p);
}

/// Gives back what it was given.
fn same(p: (u8, u8)) -> (u8, u8) {
	p
}

/// Changes what it was given in place, which is its own.
fn bump(mut h: Holder) -> u8 {
	h.pair.0 ^= 1;
	h.pair.0
}

/// Holds `pair`, for its caller to change.
fn hold(pair: (u8, u8)) -> Holder {
	Holder { pair }
}

/// A `Copy` aggregate a std call stores by value is the container's own copy: changing
/// one stored element leaves the value it came from and the other elements as they were.
pub fn stores(data: &[u8], out: &mut Vec<u8>) {
	let a = data.first().copied().unwrap_or(0);
	let b = data.get(1).copied().unwrap_or(0);
	// Made by a call, so its fields are read from it, not from `a` and `b`.
	let pair = same((a, b));
	let mut changed = (b, a);
	changed.0 ^= 0x10;

	let mut v = Vec::new();
	v.push(pair);
	v.push(pair);
	v[0].0 ^= 1;
	// A local changed in place, before and after it is stored.
	v.push(changed);
	changed.1 ^= 0x20;
	v[2].0 ^= 4;
	// Through a parameter, and through a call's result.
	keep(&mut v, pair);
	v[3].1 ^= 8;
	v.push(same(pair));
	v[4].0 ^= 0x40;
	// Each element of an array std takes apart.
	v.extend([pair, changed]);
	v[5].1 ^= 0x80;
	v[6].0 ^= 0x80;
	let old = std::mem::replace(&mut v[0], pair);
	v[0].1 ^= 0x10;
	out.extend(v.iter().flat_map(|p| [p.0, p.1]));
	out.extend_from_slice(&[changed.0, changed.1, old.0, old.1]);

	let tally = Tally { n: a as u16 };
	let mut q = VecDeque::new();
	q.push_back(tally);
	q.push_front(tally);
	q.insert(1, tally);
	if let Some(t) = q.front_mut() {
		t.n ^= 1;
	}
	if let Some(t) = q.back_mut() {
		t.n ^= 2;
	}
	out.extend(q.iter().map(|t| t.n as u8));
	out.push(tally.n as u8);

	let mut hash = HashMap::new();
	hash.insert(0u8, pair);
	hash.insert(1, pair);
	if let Some(p) = hash.get_mut(&0) {
		p.0 ^= 1;
	}
	hash.entry(2).or_insert(pair).1 ^= 2;
	out.extend((0..3).flat_map(|k| hash.get(&k).map_or([0xff; 2], |p| [p.0, p.1])));
	let mut tree = BTreeMap::new();
	tree.insert(0u8, pair);
	tree.insert(1, pair);
	if let Some(p) = tree.get_mut(&1) {
		p.1 ^= 1;
	}
	out.extend(tree.values().flat_map(|p| [p.0, p.1]));

	let mut boxed = Box::new(pair);
	boxed.0 ^= 1;
	let mut holders = vec![Holder { pair }];
	let holder = Holder { pair };
	holders.push(holder);
	holders[1].pair.1 ^= 2;
	out.extend_from_slice(&[boxed.0, boxed.1, holders[0].pair.1, holders[1].pair.1]);
	out.push(bump(Holder { pair }));
	let mut held = hold(pair);
	held.pair.1 ^= 4;
	holders.push(hold(pair));
	holders[2].pair.0 ^= 8;
	out.extend_from_slice(&[held.pair.1, holders[2].pair.0]);
	out.extend_from_slice(&[pair.0, pair.1]);
}
