//! What std's iterators drop of the values they own: the items a `for` leaves, what
//! adaptors discard or hold, what `collect` and `extend` drop as they go, and what
//! draining a collection leaves in it.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use super::Key;
use crate::log::Log;
use crate::model::Guard;

/// What `for` leaves of what it iterates, and what adaptors discard, hold, and return.
pub(super) fn looped(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	for g in list![guard(1), guard(2), guard(3)] {
		if g.id() == id + 2 {
			break;
		}
	}
	for g in [guard(4), guard(5), guard(6)] {
		log.note(format!("array {}", g.id()));
	}
	for g in Some(guard(7)) {
		log.note(format!("option {}", g.id()));
	}
	let even: Vec<Guard> = list![guard(8), guard(9), guard(10), guard(11)]
		.into_iter()
		.filter(|g| g.id() % 2 == 0)
		.collect();
	log.note(format!("even {}", even.len()));
	let mut rest = list![guard(12), guard(13), guard(14), guard(15), guard(16)]
		.into_iter()
		.skip(1)
		.take(3);
	log.note(format!("next {}", rest.next().map_or(0, |g| g.id())));
	log.note(format!("count {}", rest.count()));
	let mut peek = list![guard(17), guard(18)].into_iter().peekable();
	let first = peek.next_if(|g| g.id() == id + 17);
	let second = peek.next_if(|g| g.id() == id + 99);
	log.note(format!("peek {} {}", first.is_some(), second.is_some()));
	drop(peek);
	let pairs = list![guard(20), guard(21), guard(22)].into_iter().zip(list![guard(23)]);
	log.note(format!("zip {}", pairs.count()));
	let nth = list![guard(24)]
		.into_iter()
		.chain(list![guard(25), guard(26)])
		.rev()
		.nth(1);
	log.note(format!("nth {}", nth.map_or(0, |g| g.id())));
	let best = list![guard(27), guard(28), guard(29)]
		.into_iter()
		.max_by_key(|g| g.id() % 2);
	let found = list![Some(guard(30)), None, Some(guard(31)), Some(guard(32))]
		.into_iter()
		.flatten()
		.find(|g| g.id() == id + 31);
	log.note(format!("found {} {}", best.map_or(0, |g| g.id()), found.is_some()));
	let folded = list![guard(33), guard(34), guard(35)]
		.into_iter()
		.fold(0, |n, g| n + g.id() - id);
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

/// What `collect` drops: in place of the `Vec` its items come from, and else the source
/// first; the values before the first `Err` or `None`, and the rest; and of equal keys,
/// the one a map or set does not keep.
pub(super) fn collected(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let key = |n: u32| Key::new(id + n, log.clone());
	let same: Vec<Guard> = list![guard(1), guard(2), guard(3)].into_iter().collect();
	// Larger than what it is made from, so not in place.
	let wider: Vec<(Guard, u32)> = list![guard(4), guard(5), guard(6)]
		.into_iter()
		.map(|g| (g, 0))
		.collect();
	let chained: Vec<Guard> = list![guard(7)].into_iter().chain(list![guard(8), guard(9)]).collect();
	log.note(format!("vecs {} {} {}", same.len(), wider.len(), chained.len()));
	let results: Result<Vec<Guard>, Guard> = list![guard(10), guard(11), guard(12)]
		.into_iter()
		.map(|g| if g.id() == id + 11 { Err(g) } else { Ok(g) })
		.collect();
	log.note(format!("result {}", results.is_ok()));
	drop(results);
	let options: Option<Vec<Guard>> = list![guard(13), guard(14), guard(15)]
		.into_iter()
		.map(|g| Some(g))
		.collect();
	log.note(format!("option {}", options.map_or(0, |v| v.len())));
	let tree: BTreeMap<u32, Guard> = list![(2, guard(16)), (1, guard(17)), (2, guard(18))]
		.into_iter()
		.collect();
	log.note(format!("tree {}", tree.len()));
	// Collected references to a value that drops: each item drops as the map reads it,
	// not what the references point to.
	let refs: Vec<&Guard> = list![guard(24), guard(25)].into_iter().map(|_| &same[0]).collect();
	log.note(format!("refs {}", refs.len()));
	let queue: VecDeque<Guard> = list![guard(19), guard(20)].into_iter().collect();
	let hashed: HashMap<u32, Guard> = list![(1, guard(21)), (1, guard(22))].into_iter().collect();
	let set: HashSet<Key> = list![key(23), key(23)].into_iter().collect();
	log.note(format!("collected {} {} {}", queue.len(), hashed.len(), set.len()));
}

/// What draining a collection drops: the items taken, the half of each entry
/// `into_keys` or `into_values` leaves, and what is left when the iterator drops.
pub(super) fn drained(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let key = |n: u32| Key::new(id + n, log.clone());
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
	drop(it);
	drop(back);
	let keys: Vec<Key> = tree(5).into_keys().collect();
	let mut values = tree(9).into_values();
	log.note(format!("values {} {}", keys.len(), values.next().map_or(0, |g| g.id())));
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
	log.note(format!("entry {}", entries.next().is_some()));
	let mut map = HashMap::new();
	map.insert(key(21), guard(21));
	drop(map.drain());
	log.note(format!("drained {}", map.len()));
	let mut set = HashSet::new();
	set.insert(key(22));
	let mut taken = set.drain();
	log.note(format!("taken {}", taken.next().map_or(0, |k| k.id)));
	// Many, read by a sink in an order it ignores.
	let sink = Log::new();
	let mut map = HashMap::new();
	for n in 23..27 {
		map.insert(n, Guard::new(id + n, sink.clone()));
	}
	let mut values = map.into_values();
	let one = values.next();
	log.note(format!("rest {} {}", values.count(), one.is_some()));
	drop(one);
	log.note(format!(
		"sink {}",
		sink.text().iter().map(|&b| u32::from(b)).sum::<u32>()
	));
}

/// What `extend` drops: what a map or set does not keep of equal keys.
pub(super) fn extended(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let key = |n: u32| Key::new(id + n, log.clone());
	let mut list = list![guard(1)];
	list.extend(list![guard(2), guard(3), guard(4)]);
	list.extend(
		list![guard(5), guard(6), guard(7)]
			.into_iter()
			.filter(|g| g.id() != id + 5),
	);
	log.note(format!("list {}", list.len()));
	let mut map = HashMap::new();
	map.insert(key(8), guard(8));
	map.extend(list![(key(8), guard(9)), (key(8), guard(10))]);
	let mut set = HashSet::new();
	set.insert(key(11));
	set.extend(list![key(11), key(11)]);
	log.note(format!("hashed {} {}", map.len(), set.len()));
	let mut tree = BTreeMap::new();
	tree.insert(key(12), guard(12));
	tree.extend(list![(key(13), guard(13)), (key(12), guard(14)), (key(15), guard(15))]);
	let mut ordered = BTreeSet::new();
	ordered.insert(key(16));
	ordered.extend(list![key(17), key(16)]);
	let mut queue = VecDeque::new();
	queue.push_back(guard(18));
	queue.extend(list![guard(19), guard(20)]);
	log.note(format!("ordered {} {} {}", tree.len(), ordered.len(), queue.len()));
}
