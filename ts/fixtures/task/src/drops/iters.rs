//! What std's iterators drop of the values they own: the items a `for` leaves, what
//! adaptors discard or hold, what `collect` and `extend` drop as they go or as a panic
//! unwinds, and what draining a collection leaves in it.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use super::{Bomb, Key};
use crate::log::Log;
use crate::model::Guard;

/// What `for` leaves of what it iterates, and what adaptors discard, hold, and return,
/// panicking at point `at`, if any, some inside an adaptor's closure.
pub(super) fn looped(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	for g in list![guard(1), guard(2), guard(3)] {
		if g.id() == id + 2 {
			break;
		}
	}
	for g in [guard(4), guard(5), guard(6)] {
		log.note(format!("array {}", g.id()));
		if g.id() == id + 5 {
			check(1);
		}
	}
	for g in Some(guard(7)) {
		log.note(format!("option {}", g.id()));
	}
	let even: Vec<Guard> = list![guard(8), guard(9), guard(10), guard(11)]
		.into_iter()
		.filter(|g| {
			if g.id() == id + 10 {
				check(2);
			}
			g.id() % 2 == 0
		})
		.collect();
	log.note(format!("even {}", even.len()));
	let mut rest = list![guard(12), guard(13), guard(14), guard(15), guard(16)]
		.into_iter()
		.skip(1)
		.take(3);
	log.note(format!("next {}", rest.next().map_or(0, |g| g.id())));
	check(3);
	log.note(format!("count {}", rest.count()));
	let mut peek = list![guard(17), guard(18)].into_iter().peekable();
	let first = peek.next_if(|g| g.id() == id + 17);
	let second = peek.next_if(|g| g.id() == id + 99);
	log.note(format!("peek {} {}", first.is_some(), second.is_some()));
	check(4);
	drop(peek);
	let pairs = list![guard(20), guard(21), guard(22)].into_iter().zip(list![guard(23)]);
	log.note(format!("zip {}", pairs.count()));
	let nth = list![guard(24)]
		.into_iter()
		.chain(list![guard(25), guard(26)])
		.rev()
		.nth(1);
	log.note(format!("nth {}", nth.map_or(0, |g| g.id())));
	check(5);
	let best = list![guard(27), guard(28), guard(29)]
		.into_iter()
		.max_by_key(|g| g.id() % 2);
	let found = list![Some(guard(30)), None, Some(guard(31)), Some(guard(32))]
		.into_iter()
		.flatten()
		.find(|g| g.id() == id + 31);
	log.note(format!("found {} {}", best.map_or(0, |g| g.id()), found.is_some()));
	check(6);
	let folded = list![guard(33), guard(34), guard(35)].into_iter().fold(0, |n, g| {
		if g.id() == id + 34 {
			check(7);
		}
		n + g.id() - id
	});
	let last = list![guard(36), guard(37), guard(38)].into_iter().step_by(2).last();
	log.note(format!("fold {folded} {}", last.map_or(0, |g| g.id())));
	// What drops with a `flatten`: its source, then what it read of the front and back.
	let mut flat = list![
		list![guard(39), guard(40)],
		list![guard(41)],
		list![guard(42), guard(43)]
	]
	.into_iter()
	.flatten();
	let front = flat.next().map_or(0, |g| g.id());
	let back = flat.next_back().map_or(0, |g| g.id());
	log.note(format!("flat {front} {back}"));
	drop(flat);
	// What reading past items skips, from either end, of `skip`, `take`, and `step_by`.
	let five = |n: u32| list![guard(n), guard(n + 1), guard(n + 2), guard(n + 3), guard(n + 4)];
	let skipped = five(44).into_iter().skip(2).nth_back(3).is_some();
	let stepped = five(49).into_iter().step_by(2).nth_back(1).map_or(0, |g| g.id());
	let taken = five(59)
		.into_iter()
		.take(3)
		.rev()
		.chain(Vec::new())
		.nth(1)
		.map_or(0, |g| g.id());
	let passed = five(64)
		.into_iter()
		.skip(2)
		.rev()
		.chain(Vec::new())
		.nth(2)
		.map_or(0, |g| g.id());
	let strode = five(69).into_iter().rev().step_by(2).nth(2).map_or(0, |g| g.id());
	let counted = five(74).into_iter().rev().take(3).count();
	// A count past `usize::MAX` saturates.
	let far = (0i32..10).step_by(usize::MAX).nth_back(usize::MAX).is_none();
	log.note(format!(
		"back {skipped} {stepped} {taken} {passed} {strode} {counted} {far}"
	));
}

/// What `collect` drops: in place of the `Vec` its items come from, what it collected
/// before the rest of the source as a panic unwinds, and else the source first; the
/// values before the first `Err` or `None`, and the rest; and of equal keys, the one a
/// map or set does not keep.
pub(super) fn collected(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let key = |n: u32| Key::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	let same: Vec<Guard> = list![guard(1), guard(2), guard(3)]
		.into_iter()
		.map(|g| {
			if g.id() == id + 2 {
				check(1);
			}
			g
		})
		.collect();
	// Larger than what it is made from, so not in place.
	let wider: Vec<(Guard, u32)> = list![guard(4), guard(5), guard(6)]
		.into_iter()
		.map(|g| {
			if g.id() == id + 5 {
				check(2);
			}
			(g, 0)
		})
		.collect();
	let chained: Vec<Guard> = list![guard(7)]
		.into_iter()
		.chain(list![guard(8), guard(9)])
		.map(|g| {
			if g.id() == id + 8 {
				check(3);
			}
			g
		})
		.collect();
	log.note(format!("vecs {} {} {}", same.len(), wider.len(), chained.len()));
	let results: Result<Vec<Guard>, Guard> = list![guard(10), guard(11), guard(12)]
		.into_iter()
		.map(|g| if g.id() == id + 11 { Err(g) } else { Ok(g) })
		.collect();
	log.note(format!("result {}", results.is_ok()));
	drop(results);
	let options: Option<Vec<Guard>> = list![guard(13), guard(14), guard(15)]
		.into_iter()
		.map(|g| {
			if g.id() == id + 14 {
				check(4);
			}
			Some(g)
		})
		.collect();
	log.note(format!("option {}", options.map_or(0, |v| v.len())));
	let tree: BTreeMap<u32, Guard> = list![(2, guard(16)), (1, guard(17)), (2, guard(18))]
		.into_iter()
		.collect();
	log.note(format!("tree {}", tree.len()));
	check(5);
	let queue: VecDeque<Guard> = list![guard(19), guard(20)]
		.into_iter()
		.map(|g| {
			if g.id() == id + 20 {
				check(6);
			}
			g
		})
		.collect();
	let hashed: HashMap<u32, Guard> = list![(1, guard(21)), (1, guard(22))]
		.into_iter()
		.map(|e| {
			if e.1.id() == id + 22 {
				check(7);
			}
			e
		})
		.collect();
	let set: HashSet<Key> = list![key(23), key(23)].into_iter().collect();
	log.note(format!("collected {} {} {}", queue.len(), hashed.len(), set.len()));
}

/// What draining a collection drops: the items taken, the half of each entry
/// `into_keys` or `into_values` leaves, and what is left when the iterator drops, as a
/// panic may drop it.
pub(super) fn drained(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let key = |n: u32| Key::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	let tree = |base: u32| {
		let mut map = BTreeMap::new();
		for n in base..base + 4 {
			map.insert(key(n), guard(n));
		}
		map
	};
	let mut it = tree(1).into_iter();
	let first = it.next();
	let back = it.next_back();
	drop(first);
	check(1);
	drop(it);
	drop(back);
	let keys: Vec<Key> = tree(5).into_keys().collect();
	check(2);
	let mut values = tree(9).into_values();
	log.note(format!("values {} {}", keys.len(), values.next().map_or(0, |g| g.id())));
	check(3);
	drop(values);
	let mut set = BTreeSet::new();
	set.insert(key(13));
	set.insert(key(14));
	log.note(format!("set {}", set.into_iter().next().map_or(0, |k| k.id)));
	let mut queue = VecDeque::new();
	for n in 15..20 {
		queue.push_back(guard(n));
	}
	let mut range = queue.drain(1..4);
	log.note(format!("drain {}", range.next().map_or(0, |g| g.id())));
	check(4);
	drop(range);
	log.note(format!(
		"queue {} {}",
		queue.len(),
		queue.into_iter().next().map_or(0, |g| g.id())
	));
	// One entry each, whose order std's hash table leaves no room to differ in.
	let mut map = HashMap::new();
	map.insert(key(20), guard(20));
	let mut entries = map.into_iter();
	check(5);
	log.note(format!("entry {}", entries.next().is_some()));
	let mut map = HashMap::new();
	map.insert(key(21), guard(21));
	drop(map.drain());
	log.note(format!("drained {}", map.len()));
	let mut set = HashSet::new();
	set.insert(key(22));
	let mut taken = set.drain();
	check(6);
	log.note(format!("taken {}", taken.next().map_or(0, |k| k.id)));
	// Many, read by a sink in an order it ignores.
	let sink = Log::new();
	let mut map = HashMap::new();
	for n in 23..27 {
		map.insert(n, Guard::new(id + n, sink.clone()));
	}
	let mut values = map.into_values();
	let one = values.next();
	check(7);
	log.note(format!("rest {} {}", values.count(), one.is_some()));
	drop(one);
	log.note(format!(
		"sink {}",
		sink.text().iter().map(|&b| u32::from(b)).sum::<u32>()
	));
}

/// What `extend` drops: what a map or set does not keep of equal keys, and as a panic
/// unwinds, the rest of what it extends from, and what it has added.
pub(super) fn extended(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let key = |n: u32| Key::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	let mut list = list![guard(1)];
	list.extend(list![guard(2), guard(3), guard(4)].into_iter().map(|g| {
		if g.id() == id + 3 {
			check(1);
		}
		g
	}));
	list.extend(list![guard(5), guard(6), guard(7)].into_iter().filter(|g| {
		if g.id() == id + 6 {
			check(2);
		}
		g.id() != id + 5
	}));
	log.note(format!("list {}", list.len()));
	let mut map = HashMap::new();
	map.insert(key(8), guard(8));
	map.extend(list![(key(8), guard(9)), (key(8), guard(10))].into_iter().map(|e| {
		if e.1.id() == id + 10 {
			check(3);
		}
		e
	}));
	let mut set = HashSet::new();
	set.insert(key(11));
	set.extend(list![key(11), key(11)]);
	log.note(format!("hashed {} {}", map.len(), set.len()));
	let mut tree = BTreeMap::new();
	tree.insert(key(12), guard(12));
	tree.extend(
		list![(key(13), guard(13)), (key(12), guard(14)), (key(15), guard(15))]
			.into_iter()
			.map(|e| {
				if e.1.id() == id + 14 {
					check(4);
				}
				e
			}),
	);
	check(5);
	let mut ordered = BTreeSet::new();
	ordered.insert(key(16));
	ordered.extend(list![key(17), key(16)]);
	let mut queue = VecDeque::new();
	queue.push_back(guard(18));
	queue.extend(list![guard(19), guard(20)].into_iter().map(|g| {
		if g.id() == id + 20 {
			check(6);
		}
		g
	}));
	check(7);
	log.note(format!("ordered {} {} {}", tree.len(), ordered.len(), queue.len()));
}

/// What drops after a value's `Drop` panics in an iterator, scenario `at`'s alone: the
/// rest of what the iterator holds, or what std's code leaves.
pub(super) fn bombed(log: &Log, id: u32, at: u8) {
	let bomb = |n: u32, blows: bool| Bomb {
		id: id + n,
		log: log.clone(),
		blows,
	};
	match at {
		0 => {
			let mut it = list![bomb(1, false), bomb(2, true), bomb(3, false)].into_iter();
			drop(it.next());
			drop(it);
		}
		1 => drop(list![bomb(1, true), bomb(2, false)].into_iter().skip(1).next()),
		2 => drop(
			list![bomb(1, false), bomb(2, true), bomb(3, false)]
				.into_iter()
				.filter(|b| !b.blows)
				.count(),
		),
		3 => {
			let mut map = BTreeMap::new();
			for n in 1..4 {
				map.insert(n, bomb(n, n == 2));
			}
			let mut it = map.into_iter();
			drop(it.next());
			drop(it);
		}
		4 => {
			let mut queue = VecDeque::new();
			for n in 1..5 {
				queue.push_back(bomb(n, n == 2));
			}
			drop(queue.drain(..3));
		}
		5 => drop(
			list![(1, bomb(1, true)), (1, bomb(2, false)), (2, bomb(3, false))]
				.into_iter()
				.collect::<BTreeMap<u32, Bomb>>(),
		),
		6 => drop(
			list![bomb(1, false), bomb(2, true), bomb(3, false)]
				.into_iter()
				.take(2)
				.count(),
		),
		_ => {
			let mut flat = list![list![bomb(1, false)], list![bomb(2, true), bomb(3, false)]]
				.into_iter()
				.flatten();
			drop(flat.next());
			drop(flat);
		}
	}
	log.note(format!("unreached"));
}

/// A value that logs its drop to a sink, and panics if it is the first there: which one
/// that is, std's hash tables leave unspecified, but not how many drop after.
struct First {
	id: u32,
	sink: Log,
}

impl Drop for First {
	fn drop(&mut self) {
		let first = self.sink.text().is_empty();
		self.sink.note(format!("dropped"));
		if first {
			panic!("first blew");
		}
	}
}

impl PartialEq for First {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Eq for First {}

impl std::hash::Hash for First {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}

/// Logs how many values dropped to its sink, as it drops last.
struct Count {
	sink: Log,
	log: Log,
}

impl Drop for Count {
	fn drop(&mut self) {
		let text = self.sink.text();
		let lines = text.iter().filter(|&&b| b == b'\n').count();
		self.log.note(format!("dropped {lines}"));
	}
}

/// What drops after an item's `Drop` panics inside a consumer that drops the iterator it
/// was given, scenario `at`'s alone: the rest, as std's iterator leaves it, each once.
pub(super) fn spent(log: &Log, id: u32, at: u8) {
	let sink = Log::new();
	let _count = Count {
		sink: sink.clone(),
		log: log.clone(),
	};
	let first = |n: u32| First {
		id: id + n,
		sink: sink.clone(),
	};
	match at {
		0 => {
			let mut map = HashMap::new();
			for n in 0..3 {
				map.insert(n, first(n));
			}
			drop(map.into_iter().count());
		}
		1 => {
			let mut set = HashSet::new();
			for n in 0..3 {
				set.insert(first(n));
			}
			drop(set.into_iter().count());
		}
		2 => drop(list![first(0), first(1), first(2)].into_iter().skip(1).count()),
		3 => {
			let mut peek = list![first(0), first(1), first(2)].into_iter().peekable();
			drop(peek.peek().is_some());
			drop(peek.count());
		}
		4 => drop(list![first(0)].into_iter().chain(list![first(1), first(2)]).count()),
		5 => {
			let mut map = BTreeMap::new();
			for n in 0..3 {
				map.insert(n, first(n));
			}
			drop(map.into_iter().count());
		}
		6 => {
			let mut queue = VecDeque::new();
			for n in 0..3 {
				queue.push_back(first(n));
			}
			drop(queue.into_iter().count());
		}
		_ => {
			let mut map = HashMap::new();
			for n in 0..3 {
				map.insert(n, first(n));
			}
			drop(map.drain().count());
		}
	}
	log.note(format!("unreached"));
}
