//! Where std drops values inside its iterators and formatting: what `DebugList::entries`
//! writes, a range's bounds, a range that drains or searches a collection, the half of an
//! entry a hash map's `into_keys` and `into_values` leave, and the closure `for_each` and
//! `fold` are given.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt;
use std::ops::{Bound, RangeBounds};

use super::Key;
use crate::log::Log;
use crate::model::Guard;

/// A guard written as its id.
struct Shown(Guard);

impl fmt::Debug for Shown {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.note("shown");
		write!(f, "{}", self.0.id())
	}
}

/// Writes the guards a step makes from `0..n` by `entries` of each kind of iterable.
struct Listed(u32, Log, u8);

impl fmt::Debug for Listed {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let Self(id, log, n) = self;
		let guard = |n: u8| Shown(Guard::new(id + u32::from(n), log.clone()));
		let mut list = f.debug_list();
		// Made as `next` reads each, and dropped once written.
		list.entries((0..*n).map(guard));
		// Owned by a `Vec`, and by its `IntoIter`, which drops what it holds after.
		list.entries((10..10 + *n).map(guard).collect::<Vec<_>>());
		list.entries(
			(20..20 + *n)
				.map(guard)
				.collect::<Vec<_>>()
				.into_iter()
				.map(|g| g.0.id()),
		);
		// Borrowed, which drops nothing.
		let kept: Vec<Shown> = (30..30 + *n).map(guard).collect();
		list.entries(&kept);
		list.finish()
	}
}

/// A `RangeBounds` that logs reading its bounds and its drop.
struct Spent(Guard, usize, usize);

impl RangeBounds<usize> for Spent {
	fn start_bound(&self) -> Bound<&usize> {
		self.0.note("start");
		Bound::Included(&self.1)
	}

	fn end_bound(&self) -> Bound<&usize> {
		self.0.note("end");
		Bound::Excluded(&self.2)
	}
}

/// A range whose drop panics, which shows where std drops it.
struct Bomb(usize, usize);

impl Drop for Bomb {
	fn drop(&mut self) {
		panic!("range {}..{} dropped", self.0, self.1);
	}
}

impl RangeBounds<usize> for Bomb {
	fn start_bound(&self) -> Bound<&usize> {
		Bound::Included(&self.0)
	}

	fn end_bound(&self) -> Bound<&usize> {
		Bound::Excluded(&self.1)
	}
}

/// Run scenario `arg % 10`.
pub(super) fn inside(log: &Log, id: u32, arg: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let key = |n: u32| Key::new(id + n, log.clone());
	match arg % 10 {
		0 => {
			for n in [0, 1, 3] {
				log.note(format!("listed {:?}", Listed(id, log.clone(), n)));
			}
			log.note(format!("pretty {:#?}", Listed(id, log.clone(), 2)));
		}
		1 => {
			// Each range drops its bounds in the order its fields are.
			drop(guard(1)..guard(2));
			drop(guard(3)..=guard(4));
			drop(guard(5)..);
			drop(..guard(6));
			drop(..=guard(7));
			let r = guard(8)..guard(9);
			let start = r.start;
			log.note(format!("moved {}", start.id()));
			let ranges = vec![guard(10)..guard(11), guard(12)..guard(13)];
			log.note(format!("ranges {}", ranges.len()));
		}
		2 => {
			// std drops a range once it has read its bounds, before it drains.
			let mut v: Vec<Guard> = (1..6).map(guard).collect();
			let taken: Vec<Guard> = v.drain(Spent(guard(10), 1, 3)).collect();
			log.note(format!("drained {} {}", taken.len(), v.len()));
			let mut q: VecDeque<Guard> = (6..9).map(guard).collect();
			let mut it = q.drain(Spent(guard(11), 0, 2));
			log.note(format!("deque {:?}", it.next().map(|g| g.id())));
			drop(it);
			let mut s = String::from("abcdef");
			log.note(format!("text {}", s.drain(Spent(guard(12), 1, 3)).collect::<String>()));
			s.replace_range(Spent(guard(13), 0, 1), "xy");
			s.extend_from_within(Spent(guard(14), 2, 4));
			let mut bytes = [1u8, 2, 3, 4];
			bytes.copy_within(Spent(guard(15), 0, 2), 2);
			log.note(format!("edited {s} {bytes:?}"));
			// Empty: read and dropped all the same.
			let none: Vec<Guard> = v.drain(Spent(guard(16), 1, 1)).collect();
			log.note(format!("none {}", none.len()));
		}
		3 => {
			// A B-tree drops a range once it has searched by it, or at once if empty.
			let mut map = BTreeMap::new();
			log.note(format!("empty {}", map.range(Spent(guard(1), 0, 1)).count()));
			for n in 0..4usize {
				map.insert(n, guard(10 + n as u32));
			}
			log.note(format!("map {}", map.range(Spent(guard(2), 1, 3)).count()));
			for (_, g) in map.range_mut(Spent(guard(3), 2, 9)) {
				g.note("ranged");
			}
			let set: BTreeSet<usize> = (0..5).collect();
			log.note(format!("set {}", set.range(Spent(guard(4), 3, 5)).count()));
		}
		4 => {
			// A range of keys that drop, ordered by the crate's own `Ord`.
			let mut map = BTreeMap::new();
			for n in 1..5 {
				map.insert(key(n), n);
			}
			let found: Vec<u32> = map.range(key(2)..key(4)).map(|(_, &n)| n).collect();
			log.note(format!("keys {found:?}"));
			let found = map.range(key(3)..=key(9)).count();
			let none = BTreeMap::<Key, u32>::new().range(key(5)..key(6)).count();
			log.note(format!("keys {found} {none}"));
		}
		5 => {
			// One entry each, whose order std's hash table leaves no room to differ in: `next`
			// drops the other half before it returns, and `fold` after its closure has the item.
			let map = |n: u32| HashMap::from([(key(n), guard(n))]);
			let mut keys = map(1).into_keys();
			log.note(format!("next {:?}", keys.next().map(|k| k.id - id)));
			log.note(format!("done {}", keys.next().is_none()));
			map(2).into_values().for_each(|g| g.note("each"));
			map(3)
				.into_keys()
				.fold((), |(), k| log.note(format!("fold {}", k.id - id)));
			log.note(format!("last {:?}", map(4).into_values().last().map(|g| g.id())));
			log.note(format!("count {}", map(5).into_keys().count()));
			let found = map(6).into_values().find(|g| g.id() == id + 6);
			log.note(format!("found {:?}", found.map(|g| g.id())));
			log.note(format!("sum {}", map(7).into_keys().map(|k| k.id - id).sum::<u32>()));
			let taken: Vec<Guard> = map(8).into_values().take(0).collect();
			let none = HashMap::<Key, Guard>::new().into_keys().count();
			log.note(format!("taken {} {none}", taken.len()));
		}
		6 => {
			// Each consumer's closure drops as its fold ends, before the iterator and what it
			// still holds: a `take`'s source, and an adaptor's closure, which drops first.
			let (c, d, e, f) = (guard(10), guard(11), guard(12), guard(13));
			let v: Vec<Guard> = (1..5).map(guard).collect();
			v.into_iter()
				.take(2)
				.for_each(move |g| g.note(&format!("each {}", c.id())));
			let held = guard(14);
			let n = (5..8)
				.map(guard)
				.collect::<Vec<_>>()
				.into_iter()
				.map(move |g| (g, held.id()))
				.fold(0, move |n, (g, h)| {
					g.note(&format!("fold {h} {}", d.id()));
					n + 1
				});
			log.note(format!("folded {n}"));
			// A closure that drops over items that don't, and none at all.
			[1u32, 2]
				.iter()
				.for_each(move |n| log.note(format!("plain {n} {}", e.id())));
			Vec::<Guard>::new()
				.into_iter()
				.for_each(move |g| g.note(&format!("{}", f.id())));
			// An accumulator that drops, and a closure lent, which its owner drops after.
			let kept = (8..10)
				.map(guard)
				.fold(guard(15), |kept, g| if g.id() % 2 == 0 { g } else { kept });
			log.note(format!("kept {}", kept.id()));
			let (k, mut seen) = (guard(16), 0);
			let mut count = move |g: Guard| seen += g.id() + k.id() - 2 * id;
			(17..19).map(guard).for_each(&mut count);
			log.note("lent".into());
			drop(count);
		}
		7 => {
			// A panic mid-iteration, which aborts before anything else drops.
			let c = guard(10);
			(1..4)
				.map(guard)
				.for_each(move |g| assert!(g.id() != id + 2, "g{} with {}", g.id(), c.id()));
		}
		8 => {
			// A range past the end, which panics with std's message.
			let mut v: Vec<Guard> = (1..3).map(guard).collect();
			v.drain(Spent(guard(10), 1, 4));
		}
		_ => {
			// `String`'s drop the range once they have checked its bounds, before they check
			// its char boundaries: a range in `é` that cuts it panics as it drops.
			let mut s = String::from("é");
			let bomb = Bomb(1, 2);
			match arg / 10 % 3 {
				0 => drop(s.drain(bomb)),
				1 => s.replace_range(bomb, "x"),
				_ => s.extend_from_within(bomb),
			}
		}
	}
}
