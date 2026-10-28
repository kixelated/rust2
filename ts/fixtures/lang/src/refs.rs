//! `&mut` to values JS copies, a number, an `Option`, a `Box`, a tuple, an enum, or a
//! `String`, passed on, returned, kept in a struct, captured, and replaced by std: each
//! borrows a local, a field, an element, or a map's value, and what is written through it
//! must land there.

use std::collections::{BTreeMap, HashMap};

fn bump(x: &mut u32, by: u8) {
	*x += u32::from(by);
}

/// A parameter passed on twice, and once from a closure that captures it.
fn bump_thrice(x: &mut u32, by: u8) {
	bump(x, by);
	bump(&mut *x, 1);
	let mut again = |n: u8| bump(x, n);
	again(by);
}

/// Over 2^53, where the runtime switches a `u64` to limbs.
fn shift_wide(x: &mut u64, by: u8) {
	*x = x.wrapping_add(1 << 53).wrapping_add(u64::from(by));
}

/// moq-net's cache shape: the largest seen so far, read, replaced, and taken.
fn take_largest(seen: &mut Option<u8>, next: u8) -> Option<u8> {
	if seen.map_or(true, |s| next > s) {
		return std::mem::replace(seen, Some(next));
	}
	if let Some(s) = seen {
		*s = s.wrapping_add(1);
	}
	if next == 0 { seen.take() } else { None }
}

/// A name replaced through a parameter, its old one returned.
fn rename(name: &mut Option<String>, b: u8) -> Option<String> {
	name.replace(char::from(b'a' + b % 26).to_string())
}

/// A `&mut Option` passed on from inside a closure std calls.
fn largest_of(seen: &mut Option<u8>, items: &[u8]) -> u8 {
	let mut taken = 0u8;
	items.iter().for_each(|&b| {
		if take_largest(seen, b).is_some() {
			taken = taken.wrapping_add(1);
		}
	});
	taken
}

/// moq-net's `&mut Box<Subscriber>`: changed through, and replaced whole.
fn rebox(b: &mut Box<Pair>, by: u8) {
	b.n = b.n.wrapping_add(by);
	if by % 2 == 0 {
		*b = Box::new(Pair {
			n: b.n.wrapping_add(b.n),
		});
	}
}

fn swap_pair(p: &mut (u8, u8)) {
	std::mem::swap(&mut p.0, &mut p.1);
	p.1 = p.1.wrapping_add(1);
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Fetch {
	Idle,
	Pending(u8),
	Refused,
}

fn refuse(f: &mut Fetch) {
	*f = match *f {
		Fetch::Idle => Fetch::Pending(1),
		Fetch::Pending(n) if n > 2 => Fetch::Refused,
		Fetch::Pending(n) => Fetch::Pending(n + 1),
		Fetch::Refused => Fetch::Idle,
	};
}

fn push_to(s: &mut String, c: char) {
	s.push(c);
	if c == 'z' {
		let old = std::mem::take(s);
		for c in old.chars().take(2) {
			s.push(c);
		}
	}
}

struct Counts {
	hits: usize,
	misses: usize,
}

impl Counts {
	/// A `&mut` returned to one of two fields, as moq-net's `count` returns one.
	fn count(&mut self, hit: bool) -> &mut usize {
		match hit {
			true => &mut self.hits,
			false => &mut self.misses,
		}
	}
}

/// A struct that keeps a `&mut` to its position.
struct Cursor<'a> {
	at: &'a mut usize,
	data: &'a [u8],
}

impl Cursor<'_> {
	fn next(&mut self) -> Option<u8> {
		let b = *self.data.get(*self.at)?;
		*self.at += 1;
		Some(b)
	}
}

fn skip(remaining: &mut usize, data: &[u8]) -> usize {
	let mut skipped = 0;
	for &b in data {
		if *remaining == 0 {
			break;
		}
		*remaining -= 1;
		skipped += usize::from(b % 2);
	}
	skipped
}

struct Pair {
	n: u8,
}

/// Places std reads through: its impls for references compare, order, and format what
/// they point at.
fn read_through(a: &mut u32, b: &mut u32, out: &mut Vec<u8>) {
	out.push(u8::from(a == b));
	out.push(u8::from(a < b));
	out.push(a.cmp(&b) as u8);
	out.extend_from_slice(format!("{a}|{b:?}").as_bytes());
}

/// A `&mut Box` dereferenced through `DerefMut`, as generic code over it would.
fn deref_box(b: &mut Box<Option<u8>>, to: u8) {
	let r = <Box<Option<u8>> as std::ops::DerefMut>::deref_mut(b);
	*r = Some(to);
}

/// Methods that take `&self`, called through a `&mut`.
fn fill(seen: &mut Option<u8>, s: &mut String, next: u8) -> u8 {
	if seen.is_none() {
		*seen = Some(next);
	}
	seen.unwrap_or(0xee).wrapping_add(s.len() as u8)
}

pub fn refs(data: &[u8], out: &mut Vec<u8>) {
	let by = data.first().copied().unwrap_or(3);
	// A local, a field, and elements of an array and a `Vec`.
	let mut local = 7u32;
	bump_thrice(&mut local, by);
	let mut counts = Counts { hits: 0, misses: 0 };
	let mut held = [1u32, 2, 3];
	let mut v: Vec<u32> = data.iter().map(|&b| u32::from(b)).collect();
	for (i, &b) in data.iter().enumerate() {
		*counts.count(b % 3 == 0) += 1;
		let c = counts.count(b % 2 == 0);
		bump_slot(c, b);
		bump(&mut held[i % 3], b);
		bump(&mut v[i], 1);
	}
	out.extend_from_slice(&local.to_be_bytes());
	out.extend_from_slice(&(counts.hits as u32).to_be_bytes());
	out.extend_from_slice(&(counts.misses as u32).to_be_bytes());
	for x in held.iter().chain(&v) {
		out.extend_from_slice(&x.to_be_bytes());
	}
	// Wide integers, in a local and a field.
	let mut wide = u64::MAX - 3;
	shift_wide(&mut wide, by);
	out.extend_from_slice(&wide.to_be_bytes());
	// `Option`s, in a local and through a field's payload.
	let mut seen = None;
	out.push(largest_of(&mut seen, data));
	out.push(seen.unwrap_or(0xee));
	let mut slot = Some(by);
	if let Some(s) = &mut slot {
		bump_byte(s);
	}
	out.push(slot.unwrap_or(0xee));
	// Replaced, in a local, through a parameter, and where `None` is `rt.None`.
	let mut last = None;
	let mut name = None;
	for &b in data {
		out.push(last.replace(b).unwrap_or(0xee));
		out.push(rename(&mut name, b).map_or(0xee, |n| n.as_bytes()[0]));
	}
	let mut nested: Option<Option<u8>> = None;
	let show = |o: Option<Option<u8>>| o.map_or(0xdd, |o| o.unwrap_or(0xee));
	out.push(show(nested.replace(Some(by))));
	out.push(show(nested.replace(None)));
	out.push(show(nested));
	// A box, a tuple, an enum, and a string.
	let mut boxed = Box::new(Pair { n: by });
	let mut pair = (by, by.wrapping_add(1));
	let mut fetch = Fetch::Idle;
	let mut text = String::new();
	for &b in data {
		rebox(&mut boxed, b);
		swap_pair(&mut pair);
		refuse(&mut fetch);
		push_to(&mut text, char::from(b'a' + b % 26));
	}
	out.extend_from_slice(&[boxed.n, pair.0, pair.1]);
	out.push(match fetch {
		Fetch::Idle => 0,
		Fetch::Pending(n) => n,
		Fetch::Refused => 0xff,
	});
	out.extend_from_slice(text.as_bytes());
	let mut empty = None;
	out.push(fill(&mut empty, &mut text, by));
	out.push(fill(&mut empty, &mut text, by.wrapping_add(1)));
	// A struct keeping a `&mut`, and one passed on in a loop.
	let mut at = 0;
	let mut cursor = Cursor { at: &mut at, data };
	let mut sum = 0u8;
	while let Some(b) = cursor.next() {
		sum = sum.wrapping_add(b);
		if b > 200 {
			break;
		}
	}
	out.push(sum);
	out.push(at as u8);
	let mut remaining = usize::from(by % 5);
	out.push(skip(&mut remaining, data) as u8);
	out.push(remaining as u8);
	// A borrow through a reference reassigned while the borrow lives.
	let (mut a, mut b) = (Pair { n: 1 }, Pair { n: 2 });
	let mut p = &mut a;
	let r = &mut p.n;
	p = &mut b;
	bump_byte(r);
	p.n += 10;
	out.extend_from_slice(&[a.n, b.n]);
	// A map's values, passed on.
	let mut hash: HashMap<u8, u32> = HashMap::new();
	let mut tree: BTreeMap<u8, u32> = BTreeMap::new();
	for &b in data {
		bump(hash.entry(b % 4).or_insert(0), b);
		bump(tree.entry(b % 3).or_default(), 1);
		if let Some(x) = hash.get_mut(&(b % 2)) {
			bump(x, 2);
		}
	}
	let sorted: BTreeMap<u8, u32> = hash.into_iter().collect();
	for (k, x) in sorted.into_iter().chain(tree) {
		out.push(k);
		out.extend_from_slice(&x.to_be_bytes());
	}
	// A closure that takes a `&mut`.
	let mut total = 0u32;
	let add = |x: &mut u32, b: u8| bump(x, b);
	for &b in data {
		add(&mut total, b);
	}
	out.extend_from_slice(&total.to_be_bytes());
	// Read through by std, and dereferenced through `DerefMut`.
	let (mut x, mut y) = (u32::from(by), u32::from(data.get(1).copied().unwrap_or(5)));
	read_through(&mut x, &mut y, out);
	out.extend_from_slice(&[x as u8, y as u8]);
	let mut boxed = Box::new(None);
	deref_box(&mut boxed, total as u8);
	out.push(std::ops::Deref::deref(&boxed).unwrap_or(0xee));
}

fn bump_slot(c: &mut usize, b: u8) {
	*c += usize::from(b % 4);
}

fn bump_byte(x: &mut u8) {
	*x = x.wrapping_add(1);
}
