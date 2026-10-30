//! What `Hash` writes of std's types and the crate's into a `DefaultHasher` keyed as
//! `DefaultHasher::new` keys it, whose values must be native's byte for byte, and
//! `RandomState`, whose keys are random on both sides: its case prints only what holds
//! whatever they are, such as one state hashing equal values alike, and never a hash.

use std::cmp::Reverse;
use std::hash::{BuildHasher, DefaultHasher, Hash, Hasher, RandomState};
use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

#[derive(Hash)]
struct Packet {
	id: u64,
	flags: u8,
	name: String,
	tags: Vec<u16>,
	next: Option<i32>,
}

#[derive(Hash)]
enum Shape {
	Dot,
	Line(u32),
	Rect { w: i16, h: i16 },
}

/// A fieldless enum, which is a JS string, hashed by its `isize` discriminant.
#[derive(Clone, Copy, Hash)]
enum Dir {
	Up,
	Down,
}

/// A `u8` discriminant, which the derive writes as a byte.
#[derive(Clone, Copy, Hash)]
#[repr(u8)]
enum Small {
	Low = 3,
	High = 200,
}

/// A key whose `Hash` sees only its low nibble.
struct Nibble(u8);

impl Hash for Nibble {
	fn hash<H: Hasher>(&self, state: &mut H) {
		(self.0 & 0xf).hash(state);
	}
}

/// `DefaultHasher::new`'s hash of `value`, little-endian.
fn digest<T: Hash + ?Sized>(value: &T, out: &mut Vec<u8>) {
	let mut h = DefaultHasher::new();
	value.hash(&mut h);
	out.extend_from_slice(&h.finish().to_le_bytes());
}

/// The bytes of `data` from `at`, as many as fit, as one big-endian number.
fn wide(data: &[u8], at: usize, n: usize) -> u128 {
	let mut v = 0u128;
	for i in at..at + n {
		v = (v << 8) | data.get(i).copied().unwrap_or(0) as u128;
	}
	v
}

/// The hash of a value of each type `kind` picks, made of `data`.
pub fn values(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let w = |at: usize, n: usize| wide(data, at, n);
	match kind % 5 {
		0 => {
			let (a, b) = (w(0, 16), w(3, 8) as u64);
			digest(&(a as u8), out);
			digest(&(a as u16), out);
			digest(&(a as u32), out);
			digest(&b, out);
			// Past 2^53 or not, by the top byte.
			digest(&(b >> (data.len() % 64)), out);
			digest(&a, out);
			digest(&(a as i8), out);
			digest(&(a as i16), out);
			digest(&(a as i32), out);
			digest(&(b as i64), out);
			digest(&((b as i64) >> 11), out);
			digest(&(a as i128), out);
			digest(&((a as i128) >> 70), out);
			digest(&(a as usize), out);
			digest(&(a as isize), out);
			digest(&(a & 1 == 1), out);
			digest(&char::from_u32(a as u32 & 0x1fffff).unwrap_or('\u{10ffff}'), out);
		}
		1 => {
			let text = std::str::from_utf8(data).unwrap_or("é ✓ 😀");
			digest(text, out);
			digest(&text.to_string(), out);
			digest("", out);
			digest(&(text, text.len()), out);
			let arc: Arc<str> = Arc::from(text);
			digest(&arc, out);
		}
		2 => {
			let n = w(0, 4) as u32;
			digest(&Some(n), out);
			digest(&None::<u32>, out);
			digest(&Some(Some(n as u8)), out);
			digest(&(n as u8, "pair", w(4, 8) as u64), out);
			let words: Vec<u16> = data.iter().map(|&b| b as u16 * 257).collect();
			digest(&words, out);
			digest(&words[words.len() / 2..], out);
			digest(&[n, n ^ 1, n >> 3], out);
			digest(&(n as u8).cmp(&(w(4, 1) as u8)), out);
			digest(&Duration::new(w(0, 6) as u64, n % 1_000_000_000), out);
			digest(&Reverse(n as u16), out);
			digest(&NonZero::new(n | 1), out);
		}
		3 => {
			let n = w(0, 8) as u64;
			let packet = Packet {
				id: n,
				flags: data.first().copied().unwrap_or(0),
				name: std::str::from_utf8(data).unwrap_or("?").to_string(),
				tags: data.iter().map(|&b| b as u16).collect(),
				next: (n % 3 != 0).then_some(n as i32),
			};
			digest(&packet, out);
			let shape = match n % 3 {
				0 => Shape::Dot,
				1 => Shape::Line(n as u32),
				_ => Shape::Rect {
					w: n as i16,
					h: (n >> 16) as i16,
				},
			};
			digest(&shape, out);
			digest(&[Dir::Up, Dir::Down], out);
			digest(&(Small::Low, Small::High), out);
			digest(&Nibble(n as u8), out);
		}
		_ => {
			// Writes of each width, which fill and split the hasher's 8-byte tail, and
			// `finish` between them, which leaves the hasher as it was.
			let mut h = DefaultHasher::new();
			for (i, &b) in data.iter().enumerate() {
				let v = w(i, 16);
				match b % 13 {
					0 => h.write_u8(v as u8),
					1 => h.write_u16(v as u16),
					2 => h.write_u32(v as u32),
					3 => h.write_u64(v as u64),
					4 => h.write_u128(v),
					5 => h.write_usize(v as usize),
					6 => h.write_i8(v as i8),
					7 => h.write_i16(v as i16),
					8 => h.write_i32(v as i32),
					9 => h.write_i64(v as i64),
					10 => h.write_i128(v as i128),
					11 => h.write_isize(v as isize),
					_ => out.extend_from_slice(&h.finish().to_le_bytes()),
				}
			}
			out.extend_from_slice(&h.finish().to_le_bytes());
		}
	}
}

/// What holds of `RandomState` whatever its keys: one state, its clone, and the hasher it
/// builds hash a value alike, through a reference too, and values `Hash` sees as equal
/// alike, so ordering by its hashes leaves the same set.
pub fn random(data: &[u8], out: &mut Vec<u8>) {
	#[derive(Default)]
	struct Priorities {
		state: RandomState,
	}
	let s = RandomState::new();
	let copy = s.clone();
	let other = Priorities::default();
	for (i, &b) in data.iter().enumerate() {
		let n = wide(data, i, 8) as u64;
		let text = std::str::from_utf8(&data[i..]).unwrap_or("✓");
		let mut built = s.build_hasher();
		n.hash(&mut built);
		out.push((s.hash_one(n) == built.finish()) as u8);
		out.push((s.hash_one(n) == copy.hash_one(n)) as u8);
		out.push((s.hash_one(n) == s.hash_one(&n)) as u8);
		out.push((other.state.hash_one(text) == other.state.hash_one(text.to_string())) as u8);
		out.push((s.hash_one((b, text)) == copy.hash_one(&(b, text))) as u8);
		out.push((s.hash_one(Nibble(b)) == s.hash_one(Nibble(b ^ 0xf0))) as u8);
	}
	let mut keys: Vec<u64> = data.iter().map(|&b| b as u64 * 0x0101_0101_0101).collect();
	keys.sort_by_key(|k| other.state.hash_one(k));
	keys.sort();
	for k in keys {
		out.extend_from_slice(&k.to_le_bytes());
	}
}
