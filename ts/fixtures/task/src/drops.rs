//! Values that drop inside what std holds them in: an enum's, which has no
//! `[Symbol.dispose]` of its own, in each std type that drops what it holds, and generic
//! types that implement `Drop`. Each step logs every drop, so the log shows the order,
//! and a step that panics shows what unwinding dropped.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque, btree_map, hash_map};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use crate::kio::Consumer;
use crate::log::Log;
use crate::model::Guard;

/// A `Vec` of the items, pushed in order: `vec!` of values that are not `Copy` moves them
/// through a boxed array, which the translation lacks.
macro_rules! list {
	($($item:expr),* $(,)?) => {{
		let mut v = Vec::new();
		$(v.push($item);)*
		v
	}};
}

/// A value of a type that drops, with no `[Symbol.dispose]` of its own.
pub enum Held {
	One(Guard),
	Two(Guard, Guard),
	Empty,
}

/// Logs its own drop before its fields', whatever it holds.
pub struct Wrap<T> {
	id: u32,
	log: Log,
	inner: T,
}

impl<T> Drop for Wrap<T> {
	fn drop(&mut self) {
		self.log.note(format!("w{} drop", self.id));
	}
}

/// A generic type without a `Drop` impl, dropping its fields in order.
pub struct Pair<T> {
	first: T,
	second: Guard,
}

/// Run scenario `arg % 7`, panicking at the point `arg / 7 % 8` names, if any: what
/// every value std holds drops, where Rust drops it, and in Rust's order.
pub fn run(log: &Log, id: u32, arg: u8) {
	let at = arg / 7 % 8;
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	match arg % 7 {
		4 => ordered(log, id, at),
		5 => hashed(log, id, at),
		6 => options(log, id, at),
		0 => {
			let held = list![Held::One(guard(1)), Held::Empty, Held::Two(guard(2), guard(3))];
			check(1);
			let arc = Arc::new(Held::One(guard(4)));
			let shared = arc.clone();
			drop(arc);
			log.note(format!("arc {}", Arc::strong_count(&shared)));
			check(2);
			let mutex = Mutex::new(list![Held::One(guard(5))]);
			let boxed = Box::new(Held::Two(guard(6), guard(7)));
			let maybe = Some(Box::new(Held::One(guard(8))));
			check(3);
			log.note(format!("held {} {}", held.len(), maybe.is_some()));
			drop(boxed);
			check(4);
			drop(mutex);
			check(5);
		}
		1 => {
			let wrapped = Wrap {
				id: id + 10,
				log: log.clone(),
				inner: Held::One(guard(11)),
			};
			let guarded = Wrap {
				id: id + 20,
				log: log.clone(),
				inner: guard(21),
			};
			check(1);
			let pair = Pair {
				first: Held::Two(guard(31), guard(32)),
				second: guard(33),
			};
			let pairs = list![
				Pair {
					first: guard(41),
					second: guard(42),
				},
				Pair {
					first: guard(43),
					second: guard(44),
				},
			];
			check(2);
			let nested = Wrap {
				id: id + 50,
				log: log.clone(),
				inner: list![Wrap {
					id: id + 60,
					log: log.clone(),
					inner: Held::One(guard(61)),
				}],
			};
			check(3);
			drop(pair);
			check(4);
			log.note(format!(
				"wraps {} {} {}",
				wrapped.inner.is_some(),
				guarded.inner.id(),
				pairs[1].first.id() + pairs[0].second.id() + nested.inner.len() as u32
			));
		}
		2 => {
			let shared = Arc::new(Wrap {
				id: id + 70,
				log: log.clone(),
				inner: Pair {
					first: Held::One(guard(71)),
					second: guard(72),
				},
			});
			let clones = list![shared.clone(), shared.clone()];
			check(1);
			drop(shared);
			log.note(format!("clones {}", clones.len()));
			check(2);
			let early = bail(log, id, at);
			log.note(format!("bailed {}", early.is_err()));
		}
		_ => {
			let arc = Arc::new(Mutex::new(Held::Two(guard(81), guard(82))));
			let other = Arc::clone(&arc);
			check(1);
			drop(arc);
			check(2);
			let held = other.lock().unwrap().is_some();
			log.note(format!("locked {held}"));
			// The last `Arc` of a borrow drops nothing it points at.
			let lent = guard(83);
			let borrowed = Arc::new(&lent);
			let again = Arc::clone(&borrowed);
			drop(borrowed);
			check(3);
			drop(again);
			log.note(format!("lent {}", lent.id()));
		}
	}
}

/// Returns early through `?` with an enum's guards and a generic `Drop` type live.
fn bail(log: &Log, id: u32, at: u8) -> Result<u32, u32> {
	let held = list![Held::One(Guard::new(id + 90, log.clone()))];
	let wrap = Wrap {
		id: id + 91,
		log: log.clone(),
		inner: Held::Empty,
	};
	let n = fails(at)?;
	log.note(format!("not bailed {} {}", held.len(), wrap.inner.is_some()));
	Ok(n)
}

fn fails(at: u8) -> Result<u32, u32> {
	match at % 2 {
		0 => Err(u32::from(at)),
		_ => Ok(1),
	}
}

/// Holds an enum's guards in std types and a generic `Drop` type across an await,
/// which a cancel drops.
pub async fn held(mut rx: Consumer, guard: Guard) -> u64 {
	let log = guard.log().clone();
	let id = guard.id();
	let held = list![Held::One(Guard::new(id + 500, log.clone())), Held::Empty];
	let wrap = Wrap {
		id: id + 510,
		log: log.clone(),
		inner: Arc::new(Held::One(Guard::new(id + 520, log.clone()))),
	};
	let value = rx.recv().await.unwrap_or(0);
	log.note(format!("g{id} held {} {}", held.len(), wrap.inner.is_some()));
	value
}

impl Held {
	fn is_some(&self) -> bool {
		!matches!(self, Held::Empty)
	}
}

/// A key that logs its own drop, compared, ordered, and hashed by its id alone.
pub struct Key {
	id: u32,
	log: Log,
}

impl Drop for Key {
	fn drop(&mut self) {
		self.log.note(format!("k{} drop", self.id));
	}
}

impl PartialEq for Key {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Eq for Key {}

impl PartialOrd for Key {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Key {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id.cmp(&other.id)
	}
}

impl Hash for Key {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}

/// What ordered collections drop of what they are given or hold, in order: a key
/// `insert` and `entry` find there already, the key `remove` takes out, the value
/// `or_insert` is given, and what `retain`, `truncate`, and `clear` discard.
fn ordered(log: &Log, id: u32, at: u8) {
	let key = |n: u32| Key {
		id: id + n,
		log: log.clone(),
	};
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	let mut map = BTreeMap::new();
	map.insert(key(1), guard(1));
	map.insert(key(2), guard(2));
	map.insert(key(3), guard(3));
	let old = map.insert(key(1), guard(4));
	log.note(format!("replaced {}", old.is_some()));
	drop(old);
	check(1);
	let removed = map.remove(&key(2));
	log.note(format!("removed {}", removed.is_some()));
	match map.entry(key(3)) {
		btree_map::Entry::Occupied(e) => log.note(format!("occupied {}", e.remove().id())),
		btree_map::Entry::Vacant(_) => log.note(format!("vacant")),
	}
	if let btree_map::Entry::Vacant(e) = map.entry(key(5)) {
		log.note(format!("left {}", e.key().id));
	}
	check(2);
	let kept = map.entry(key(1)).or_insert(guard(6)).id();
	let added = map.entry(key(7)).or_insert(guard(7)).id();
	log.note(format!("or_insert {kept} {added}"));
	map.insert(key(8), guard(8));
	map.retain(|k, _| k.id % 2 == 0);
	check(3);
	let mut set = BTreeSet::new();
	set.insert(key(10));
	set.insert(key(11));
	log.note(format!("again {}", set.insert(key(10))));
	log.note(format!("gone {}", set.remove(&key(11))));
	set.insert(key(12));
	check(4);
	let mut queue = VecDeque::new();
	for n in 20..26 {
		queue.push_back(guard(n));
	}
	queue.push_front(guard(19));
	queue.retain(|g| g.id() % 3 != 0);
	check(5);
	queue.truncate(2);
	log.note(format!("queue {}", queue.len()));
	map.clear();
	set.clear();
	check(6);
	log.note(format!("left {} {} {}", map.len(), set.len(), queue.len()));
}

/// The same of hash collections, whose order of drops std leaves unspecified: each key
/// and value logs to a sink, and the log reads how much it holds, in an order it ignores.
fn hashed(log: &Log, id: u32, at: u8) {
	let sink = Log::new();
	let key = |n: u32| Key {
		id: id + n,
		log: sink.clone(),
	};
	let guard = |n: u32| Guard::new(id + n, sink.clone());
	let tally = |what: &str| {
		let text = sink.text();
		let sum: u32 = text.iter().map(|&b| u32::from(b)).sum();
		log.note(format!("{what}: {} {sum}", text.len()));
	};
	let check = |point: u8| {
		if point == at {
			tally("panic");
			panic!("d{id} at {point}");
		}
	};
	let mut map = HashMap::new();
	map.insert(key(1), guard(1));
	map.insert(key(2), guard(2));
	map.insert(key(3), guard(3));
	let old = map.insert(key(1), guard(4));
	tally("insert");
	drop(old);
	check(1);
	let removed = map.remove(&key(2));
	tally("remove");
	drop(removed);
	match map.entry(key(3)) {
		hash_map::Entry::Occupied(e) => drop(e.remove()),
		hash_map::Entry::Vacant(_) => log.note(format!("vacant")),
	}
	if let hash_map::Entry::Vacant(e) = map.entry(key(5)) {
		log.note(format!("left {}", e.key().id));
	}
	tally("entry");
	check(2);
	let kept = map.entry(key(1)).or_insert(guard(6)).id();
	let added = map.entry(key(7)).or_insert(guard(7)).id();
	log.note(format!("or_insert {kept} {added}"));
	tally("or_insert");
	map.insert(key(8), guard(8));
	map.retain(|k, _| k.id % 2 == 0);
	tally("retain");
	check(3);
	let mut set = HashSet::new();
	set.insert(key(10));
	set.insert(key(11));
	log.note(format!("again {}", set.insert(key(10))));
	log.note(format!("gone {}", set.remove(&key(11))));
	set.insert(key(12));
	set.retain(|k| k.id % 2 == 1);
	tally("set");
	check(4);
	map.clear();
	tally("clear");
	check(5);
	drop(set);
	drop(map);
	tally("end");
}

/// What std's functions on `Option`, `Result`, and `bool` drop of what they are given
/// rather than return: an unused error or fallback, after any closure they call.
fn options(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	let some = Some(guard(1)).ok_or(guard(2));
	let none = if at > 8 { Some(guard(0)) } else { None }.ok_or(guard(3));
	log.note(format!("ok_or {} {}", some.is_ok(), none.is_ok()));
	check(1);
	let ok = Ok::<Guard, Guard>(guard(4)).ok();
	let err = Err::<Guard, Guard>(guard(5)).ok();
	let flipped = Ok::<Guard, Guard>(guard(6)).err();
	log.note(format!("ok {} {} {}", ok.is_some(), err.is_some(), flipped.is_some()));
	check(2);
	let kept = Some(guard(7)).unwrap_or(guard(8));
	let fallback = None.unwrap_or(guard(9));
	let value = Ok::<Guard, Guard>(guard(10)).unwrap_or(guard(11));
	let error = Err::<Guard, Guard>(guard(12)).unwrap_or(guard(13));
	log.note(format!(
		"unwrap_or {} {} {} {}",
		kept.id(),
		fallback.id(),
		value.id(),
		error.id()
	));
	check(3);
	let mapped = Some(id).map_or(guard(14), |n| {
		log.note(format!("mapped"));
		Guard::new(n + 15, log.clone())
	});
	let unmapped = None.map_or(guard(16), |n: u32| Guard::new(n, log.clone()));
	log.note(format!("map_or {} {}", mapped.id(), unmapped.id()));
	check(4);
	let then = (at % 2 == 0).then_some(guard(17));
	let never = false.then_some(guard(18));
	log.note(format!("then_some {} {}", then.is_some(), never.is_some()));
	check(5);
}
