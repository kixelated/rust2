//! `&mut` to values JS copies, a number, an `Option`, a `Box`, a tuple, an enum, or a
//! `String`, passed on, returned, kept in a struct, and captured: each borrows a local, a
//! field, an element, or a map's value, and what is written through it must land there.

use std::collections::{BTreeMap, HashMap, VecDeque};

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
	// The whole tuple, replaced in the caller's.
	if p.0 % 3 == 0 {
		*p = (p.1, p.0);
	}
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
	lent(data, out);
}

/// What std lends of an `Option` a `&mut` borrows: its payload, written through in place,
/// passed on, and kept.
fn lent(data: &[u8], out: &mut Vec<u8>) {
	let mut seen: Option<u32> = None;
	let mut last: Option<(u8, u8)> = None;
	let mut text: Option<String> = None;
	let mut held: Vec<&mut u32> = Vec::new();
	let mut spare = [None, Some(9u32)];
	for &b in data {
		match b % 6 {
			0 => *seen.get_or_insert(u32::from(b)) += 1,
			1 => bump(seen.get_or_insert_with(|| u32::from(b) * 2), b),
			2 => *seen.get_or_insert_default() += u32::from(b),
			3 => {
				let x = seen.insert(u32::from(b));
				*x = x.wrapping_add(*x);
			}
			4 => {
				if let Some(x) = seen.as_mut() {
					bump(x, b);
				}
			}
			_ => seen = None,
		}
		last.get_or_insert((b, 0)).1 ^= b;
		if let Some((x, _)) = last.as_mut() {
			*x = x.wrapping_add(1);
		}
		// Through `Option`'s combinators, which hand the closure the place.
		if let Some(p) = last.as_mut().filter(|p| p.0 % 2 == 0).map(|p| {
			p.1 = p.1.wrapping_add(2);
			p
		}) {
			p.0 ^= 1;
		}
		let doubled = seen.as_mut().and_then(|x| (*x % 3 == 0).then_some(x)).map_or(0, |x| {
			*x *= 2;
			*x
		});
		out.extend_from_slice(&doubled.to_be_bytes());
		text.get_or_insert_with(String::new).push(char::from(b'a' + b % 26));
		out.extend_from_slice(&seen.unwrap_or(0xeeee).to_be_bytes());
	}
	let (x, y) = last.unwrap_or((0xee, 0xee));
	out.extend_from_slice(&[x, y]);
	out.extend_from_slice(text.unwrap_or_default().as_bytes());
	// A payload lent through a field and an element, then kept past its loan.
	let mut counts = Pair2 { a: None, b: None };
	for (i, &b) in data.iter().enumerate() {
		let at = match i % 2 {
			0 => counts.a.get_or_insert(0),
			_ => counts.b.get_or_insert_with(|| 100),
		};
		*at += u32::from(b);
		bump(spare[i % 2].get_or_insert(1), b);
	}
	for (i, x) in spare.iter_mut().enumerate() {
		held.push(x.insert(i as u32));
	}
	for x in held {
		*x += 7;
	}
	for x in [counts.a, counts.b, spare[0], spare[1]] {
		out.extend_from_slice(&x.unwrap_or(0xeeee).to_be_bytes());
	}
	elements(data, out);
}

/// What std lends of the elements and values of slices and collections, which JS copies:
/// one, each in turn, or each to a closure.
fn elements(data: &[u8], out: &mut Vec<u8>) {
	let mut v: Vec<u32> = data.iter().map(|&b| u32::from(b)).collect();
	let mut bytes = data.to_vec();
	let mut pairs: Vec<(u8, Option<u8>)> = data.iter().map(|&b| (b, b.checked_sub(100))).collect();
	let by = data.first().copied().unwrap_or(1);
	if let Some(x) = v.first_mut() {
		*x += 1;
	}
	if let Some(x) = v.last_mut() {
		bump(x, by);
	}
	if let Some(x) = v.get_mut(usize::from(by) % 4) {
		*x *= 2;
	}
	if let Some(b) = bytes.last_mut() {
		*b = b.wrapping_add(1);
	}
	if let Some(b) = bytes.get_mut(1) {
		bump_byte(b);
	}
	for x in v.iter_mut().rev().step_by(2) {
		*x += 3;
	}
	for x in &mut v {
		bump(x, 1);
	}
	for (i, b) in bytes.iter_mut().enumerate() {
		*b ^= i as u8;
	}
	for (b, x) in bytes.iter_mut().zip(&mut v) {
		*x += u32::from(*b);
		*b = b.wrapping_add(*b);
	}
	// Places found, kept, and compared.
	if let Some((_, slot)) = pairs.iter_mut().find(|(k, _)| *k % 3 == 0) {
		*slot = Some(slot.map_or(0, |s| s.wrapping_add(1)));
	}
	let most = v.iter_mut().max_by_key(|x| **x);
	if let Some(x) = most {
		*x = 0;
	}
	let every: Vec<&mut u32> = v.iter_mut().filter(|x| **x % 2 == 0).collect();
	for x in every {
		*x += 1;
	}
	let mut grid = [[0u32; 2]; 3];
	for (row, &b) in grid.iter_mut().zip(data) {
		for (i, x) in row.iter_mut().enumerate() {
			*x = u32::from(b) << i;
		}
	}
	for x in &v {
		out.extend_from_slice(&x.to_be_bytes());
	}
	out.extend_from_slice(&bytes);
	for (k, slot) in &pairs {
		out.extend_from_slice(&[*k, slot.unwrap_or(0xee)]);
	}
	for x in grid.iter().flatten() {
		out.extend_from_slice(&x.to_be_bytes());
	}
	// Maps' and a deque's values.
	let mut hash: HashMap<u8, (u8, u8)> = HashMap::new();
	let mut tree: BTreeMap<u8, u32> = BTreeMap::new();
	let mut queue: VecDeque<u32> = VecDeque::new();
	for &b in data {
		hash.entry(b % 5)
			.and_modify(|x| x.1 = x.1.wrapping_add(b))
			.or_insert((b, 0));
		*tree.entry(b % 4).and_modify(|x| *x += 1).or_default() += u32::from(b);
		queue.push_back(u32::from(b));
		if b % 3 == 0 {
			queue.push_front(u32::from(b) + 1);
		}
	}
	for x in hash.values_mut() {
		x.0 = x.0.wrapping_add(1);
	}
	for (k, x) in hash.iter_mut() {
		if k % 2 == 0 {
			*x = (x.1, x.0);
		}
	}
	for (k, x) in &mut tree {
		*x += u32::from(*k);
	}
	for x in tree.values_mut().rev() {
		*x *= 2;
	}
	for (k, x) in tree.range_mut(1..3) {
		*x += u32::from(*k) << 8;
	}
	// An `Option` whose payload can be `undefined`, whose `None` is apart from it.
	let mut nested: Option<Option<u8>> = None;
	for &b in data {
		match b % 3 {
			0 => *nested.get_or_insert(None) = b.checked_sub(50),
			1 => {
				if let Some(x) = nested.as_mut() {
					*x = x.map(|x| x.wrapping_add(1));
				}
			}
			_ => drop(nested.insert(Some(b))),
		}
		for x in &mut nested {
			*x = x.or(Some(1));
		}
	}
	out.push(match nested {
		None => 0xee,
		Some(None) => 0xdd,
		Some(Some(x)) => x,
	});
	// A borrowed `Option` iterated: its payload's place, or nothing.
	for o in [seen_of(data), None] {
		let mut o = o;
		for x in &mut o {
			*x = x.wrapping_add(7);
		}
		out.push((&mut o).into_iter().count() as u8);
		out.push(o.unwrap_or(0xee));
	}
	for x in seen_of(data).iter_mut() {
		*x ^= 0xff;
		tree.insert(9, u32::from(*x));
	}
	hash.retain(|k, x| {
		x.1 ^= *k;
		*k != by % 5
	});
	tree.retain(|k, x| {
		*x += 1;
		*k != by % 4
	});
	for x in queue.iter_mut() {
		*x += 1;
	}
	if let Some(x) = queue.front_mut() {
		*x += 100;
	}
	if let Some(x) = queue.back_mut() {
		*x += 200;
	}
	if let Some(x) = queue.get_mut(1) {
		*x += 300;
	}
	queue.retain_mut(|x| {
		*x += 1;
		*x % 3 != 0
	});
	// Keys that are objects, which the runtime chains by hash.
	let mut paired: HashMap<(u8, u8), u32> = HashMap::new();
	for &b in data {
		*paired.entry((b % 3, b % 2)).or_default() += u32::from(b);
	}
	for (k, x) in paired.iter_mut() {
		*x += u32::from(k.0);
	}
	for x in paired.values_mut() {
		*x *= 2;
	}
	paired.retain(|k, x| {
		*x += 1;
		k.1 == 0
	});
	let paired: BTreeMap<(u8, u8), u32> = paired.into_iter().collect();
	for ((a, b), x) in paired {
		out.extend_from_slice(&[a, b]);
		out.extend_from_slice(&x.to_be_bytes());
	}
	let hash: BTreeMap<u8, (u8, u8)> = hash.into_iter().collect();
	for (k, (x, y)) in hash {
		out.extend_from_slice(&[k, x, y]);
	}
	for (k, x) in tree {
		out.push(k);
		out.extend_from_slice(&x.to_be_bytes());
	}
	for x in queue {
		out.extend_from_slice(&x.to_be_bytes());
	}
}

fn seen_of(data: &[u8]) -> Option<u8> {
	data.get(2).copied()
}

struct Pair2 {
	a: Option<u32>,
	b: Option<u32>,
}

fn bump_slot(c: &mut usize, b: u8) {
	*c += usize::from(b % 4);
}

fn bump_byte(x: &mut u8) {
	*x = x.wrapping_add(1);
}
