//! Options of values that can already be `undefined`, `Option<Option<u8>>` and
//! `Option<()>`, whose `None` is `rt.None` apart from each `Some`.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};

/// Run case `kind % 4` over `data`, whose bytes pick each `None`, `Some(None)`, or
/// `Some(Some(_))`.
pub fn nested(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 4 {
		0 => made(data, out),
		1 => keys(data, out),
		2 => iters(data, out),
		_ => panics(data, out),
	}
}

/// `None` for a byte that is 0 mod 3, `Some(None)` for 1, and else `Some(Some(byte))`.
fn pick(b: u8) -> Option<Option<u8>> {
	match b % 3 {
		0 => None,
		1 => Some(None),
		_ => Some(Some(b)),
	}
}

/// `Some(())` for an even byte.
fn unit(b: u8) -> Option<()> {
	(b % 2 == 0).then_some(())
}

/// Which of the three `o` is, and its byte.
fn code(o: Option<Option<u8>>) -> u8 {
	match o {
		None => 0xe0,
		Some(None) => 0xe1,
		Some(Some(b)) => b,
	}
}

/// `?` on an `Option<()>` and on an `Option<Option<u8>>`, which return their `None`.
fn tried(a: Option<()>, b: Option<Option<u8>>) -> Option<Option<u8>> {
	a?;
	let inner = b?;
	Some(inner.map(|x| x / 2))
}

/// A generic type over a `T` that can be `undefined`, beside a field of no type parameter.
struct Tagged<T> {
	value: T,
	order: Ordering,
}

struct Slot {
	held: Option<Option<u8>>,
	flag: Option<()>,
}

fn made(data: &[u8], out: &mut Vec<u8>) {
	for &b in data {
		let o = pick(b);
		let u = unit(b);
		out.push(code(o));
		out.push(o.is_some() as u8 | (o.is_none() as u8) << 1 | (u.is_some() as u8) << 2 | (u.is_none() as u8) << 3);
		out.push(code(o.map(|x| x.map(|y| y ^ 0x55))));
		out.push(o.flatten().unwrap_or(0xf0));
		out.push(code(o.and_then(|x| x.map(|y| Some(y.wrapping_add(1))))));
		out.push(code(o.filter(|x| x.is_some())));
		out.push(code(o.or(Some(None))));
		out.push(code(o.xor(u.map(|()| None))));
		out.push(u.map_or(0xd0, |()| 0xd1));
		out.push(code(tried(u, o)));
		let copied = o.as_ref().copied();
		out.push((copied == o) as u8);
		let mut slot = Slot { held: o, flag: u };
		let taken = slot.held.take();
		let flag = std::mem::take(&mut slot.flag);
		out.push(code(taken) ^ code(slot.held));
		out.push(flag.is_some() as u8 | (slot.flag.is_some() as u8) << 1);
		let replaced = std::mem::replace(&mut slot.held, Some(Some(b)));
		out.push(code(replaced) ^ code(slot.held));
		let default: Option<Option<u8>> = Default::default();
		out.push(code(default));
		let tagged = Tagged {
			value: u,
			order: b.cmp(&0x80),
		};
		out.push(tagged.value.is_some() as u8 | order(tagged.order) << 1);
		out.extend_from_slice(format!("{o:?} {u:?}").as_bytes());
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

/// Compared, ordered, and hashed: `None` orders before `Some(None)`, which orders before
/// each `Some(Some(_))`.
fn keys(data: &[u8], out: &mut Vec<u8>) {
	let mut hashed: HashMap<(u8, Option<Option<u8>>), u8> = HashMap::new();
	let mut sorted: BTreeMap<(Option<Option<u8>>, u8), u8> = BTreeMap::new();
	for i in 1..data.len() {
		let (a, b) = (pick(data[i - 1]), pick(data[i]));
		out.push((a == b) as u8 | ((a != b) as u8) << 1 | ((a < b) as u8) << 2 | order(a.cmp(&b)) << 3);
		out.push((a.max(b) == b) as u8 | ((hash_of(&a) == hash_of(&b)) as u8) << 1);
		out.push(hashed.insert((data[i] % 2, a), i as u8).unwrap_or(0xcc));
		out.push(sorted.insert((a, data[i] % 2), i as u8).unwrap_or(0xcc));
		out.push(hashed.contains_key(&(data[i - 1] % 2, b)) as u8);
	}
	for ((o, n), v) in &sorted {
		out.push(code(*o));
		out.push(*n);
		out.push(*v);
	}
}

/// Counts down to 0, yielding each count as an `Option<u8>`: `None` for each odd one.
struct Halves(u8);

impl Iterator for Halves {
	type Item = Option<u8>;

	fn next(&mut self) -> Option<Option<u8>> {
		let n = self.0.checked_sub(1)?;
		self.0 = n;
		halve(n)
	}
}

fn halve(n: u8) -> Option<Option<u8>> {
	Some((n % 2 == 0).then_some(n / 2))
}

fn iters(data: &[u8], out: &mut Vec<u8>) {
	let items: Vec<Option<u8>> = data.iter().map(|&b| (b % 2 == 0).then_some(b)).collect();
	let at = data.first().map_or(0, |b| usize::from(*b) % 4);
	out.push(code(items.get(at).copied()));
	out.push(code(items.first().copied()));
	out.push(code((at > 1).then(|| items.get(at).copied().flatten())));
	let mut it = items.iter().copied();
	out.push(code(it.next()));
	out.push(code(it.next_back()));
	out.push(code(it.nth_back(1)));
	for item in it {
		out.push(item.unwrap_or(0xe1));
	}
	let mut own = Halves(data.len() as u8 % 8);
	let first = own.next();
	out.push(code(first));
	for item in own {
		out.push(item.map_or(0xe1, |x| x));
	}
	for b in data {
		let o = pick(*b);
		out.push(o.iter().count() as u8);
		o.into_iter().for_each(|x| out.push(x.unwrap_or(0xe1)));
		unit(*b).into_iter().for_each(|()| out.push(0xd1));
	}
}

fn panics(data: &[u8], out: &mut Vec<u8>) {
	for &b in data {
		out.push(pick(b).expect("picked none").unwrap_or(0xe1));
		out.push(code(Some(pick(b).unwrap())));
		unit(b ^ 1).unwrap();
	}
}
