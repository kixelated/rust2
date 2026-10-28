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

mod closures;
mod iters;

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

/// Borrowed text or a value that drops, as moq-net's `path::Repr` is: generic over a
/// lifetime alone.
pub enum Text<'a> {
	Borrowed(&'a str),
	Owned(Guard),
}

/// A type generic over a lifetime alone, holding one that is.
pub struct Named<'a>(Text<'a>);

/// Pairs of its type parameters, as a map's entries are: each pair drops its halves in
/// order, the second after the first panics.
pub struct Entries<K, V>(Vec<(K, V)>);

/// Run scenario `arg % 23`, panicking at the point `arg / 23 % 11` names, if any: what
/// every value std holds drops, where Rust drops it, and in Rust's order.
pub fn run(log: &Log, id: u32, arg: u8) {
	let at = arg / 23 % 11;
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	match arg % 23 {
		22 => closures::iterated(log, id, at),
		19 => closures::scoped(log, id, at),
		20 => closures::discarded(log, id, at),
		21 => closures::blasted(log, id, at),
		13 => iters::looped(log, id, at),
		14 => iters::collected(log, id, at),
		15 => iters::drained(log, id, at),
		16 => iters::extended(log, id, at),
		17 => iters::bombed(log, id, at),
		18 => iters::spent(log, id, at),
		4 => ordered(log, id, at),
		5 => hashed(log, id, at),
		6 => options(log, id, at),
		7 => arcs(log, id, at),
		8 => blown(log, id, at),
		9 => tripped(log, id, at, true),
		10 => tripped(log, id, at, false),
		11 => grown(log, id, at, true),
		12 => grown(log, id, at, false),
		0 => {
			let held = list![Held::One(guard(1)), Held::Empty, Held::Two(guard(2), guard(3))];
			check(1);
			let arc = Arc::new(Held::One(guard(4)));
			let shared = arc.clone();
			drop(arc);
			log.note(format!("arc {}", Arc::strong_count(&shared)));
			// A borrowed count is the count when it was borrowed, though a clone follows.
			let counted = (&Arc::strong_count(&shared),);
			let count = counted.0;
			let cloned = shared.clone();
			log.note(format!("counted {}", *count));
			drop(cloned);
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
			let _borrowed = Named(Text::Borrowed("text"));
			let _owned = Named(Text::Owned(guard(45)));
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

/// A key that logs its own drop, compared, ordered, and hashed by its id alone, unless
/// its fault makes it panic there.
pub struct Key {
	id: u32,
	log: Log,
	fault: Fault,
}

/// Where a `Key` or `Bomb` panics, to show what std drops as the panic unwinds.
#[derive(Clone, Copy)]
enum Fault {
	Sound,
	/// In its `Drop`, after logging it.
	Blows,
	/// In its `Hash`, `Eq`, or `Ord`.
	Trips,
	/// Never, but its `Hash` logs each call.
	Counted,
}

impl Key {
	fn new(id: u32, log: Log) -> Self {
		Self {
			id,
			log,
			fault: Fault::Sound,
		}
	}

	fn faulty(id: u32, log: Log, fault: Fault) -> Self {
		Self { id, log, fault }
	}

	/// Panic if this key trips, as a comparison or hash reads it.
	fn read(&self) -> u32 {
		if matches!(self.fault, Fault::Trips) {
			panic!("k{} tripped", self.id);
		}
		self.id
	}
}

impl Drop for Key {
	fn drop(&mut self) {
		self.log.note(format!("k{} drop", self.id));
		// Nothing reads a dropped key in Rust; a translation that did would find another.
		self.id = u32::MAX;
		if matches!(self.fault, Fault::Blows) {
			panic!("k{} blew", self.id);
		}
	}
}

impl PartialEq for Key {
	fn eq(&self, other: &Self) -> bool {
		self.read() == other.read()
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
		self.read().cmp(&other.read())
	}
}

impl Hash for Key {
	fn hash<H: Hasher>(&self, state: &mut H) {
		if matches!(self.fault, Fault::Counted) {
			self.log.note(format!("k{} hashed", self.id));
		}
		self.read().hash(state);
	}
}

/// The same key, logged, unless it trips.
impl Clone for Key {
	fn clone(&self) -> Self {
		let id = self.read();
		self.log.note(format!("k{id} cloned"));
		Self::faulty(id, self.log.clone(), self.fault)
	}
}

impl std::fmt::Debug for Key {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("Key").field(&self.id).finish()
	}
}

/// What ordered collections drop of what they are given or hold, in order: a key
/// `insert` and `entry` find there already, the key `remove` takes out, the value
/// `or_insert` is given, and what `retain`, `truncate`, and `clear` discard.
fn ordered(log: &Log, id: u32, at: u8) {
	let key = |n: u32| Key::new(id + n, log.clone());
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
	let key = |n: u32| Key::new(id + n, sink.clone());
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
	// An entry hashes its key once, and what consumes it hashes nothing: two keys, so the
	// table never grows, which hashes each again.
	let counted = |n: u32| Key::faulty(id + n, sink.clone(), Fault::Counted);
	let mut counts = HashMap::new();
	counts.insert(counted(20), guard(20));
	drop(counts.entry(counted(20)).or_insert(guard(21)));
	drop(counts.entry(counted(22)).or_insert_with(|| guard(22)));
	drop(counts.entry(counted(20)).and_modify(|_| ()).or_insert(guard(23)));
	if let hash_map::Entry::Occupied(e) = counts.entry(counted(22)) {
		drop(e.remove());
	}
	tally("counted");
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
	// Lowered from core's MIR, whose cleanups drop what a panic in a closure leaves.
	let kept = Some(guard(19)).filter(|g| {
		check(6);
		g.id() % 2 == 0
	});
	let dropped = Some(guard(20)).filter(|_| false);
	let zipped = Some(guard(21)).zip(if at % 2 == 0 { Some(guard(22)) } else { None });
	let other = None.or_else(|| Some(guard(23)));
	let flag = Some(guard(24)).is_some_and(|g| g.id() > id);
	log.note(format!(
		"filter {} {} zip {} or_else {} is_some_and {flag}",
		kept.is_some(),
		dropped.is_some(),
		zipped.is_some(),
		other.is_some()
	));
	let chained = Ok::<Guard, Guard>(guard(25)).and_then(|g| {
		check(7);
		Err::<Guard, Guard>(g)
	});
	let error = Err::<Guard, Guard>(guard(26)).map_or_else(|e| e.id(), |v| v.id());
	let cloned = Some(Arc::new(guard(27))).clone();
	log.note(format!(
		"and_then {} map_or_else {error} clone {}",
		chained.is_ok(),
		cloned.is_some()
	));
}

/// What maps keyed by `Arc<str>` drop of the keys they are given or hold, and `max` and
/// `min` of the operand they do not return, read by the keys' counts; and what
/// `or_insert` drops of a value JS copies, which it returns a place for.
fn arcs(log: &Log, id: u32, at: u8) {
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	let count = |what: &str, key: &Arc<str>| log.note(format!("{what} {}", Arc::strong_count(key)));
	let name: Arc<str> = Arc::from("name");
	let other: Arc<str> = Arc::from("other");
	let mut map = HashMap::new();
	map.insert(name.clone(), 1u8);
	map.insert(name.clone(), 2);
	map.insert(other.clone(), 3);
	count("insert", &name);
	let mut tree = BTreeMap::new();
	tree.insert(other.clone(), 1u8);
	tree.insert(other.clone(), 2);
	count("tree", &other);
	check(1);
	*map.entry(name.clone()).or_insert(0) += 1;
	count("entry", &name);
	map.remove(&name);
	count("remove", &name);
	tree.remove("other");
	count("removed", &other);
	check(2);
	let max = name.clone().max(other.clone());
	let min = name.clone().min(other.clone());
	log.note(format!("max {} min {}", &*max, &*min));
	count("name", &name);
	count("other", &other);
	drop((max, min));
	check(3);
	let mut tags = BTreeMap::new();
	tags.insert(1u8, Held::One(Guard::new(id + 1, log.clone())));
	if let Held::One(guard) = tags.entry(1).or_insert(Held::One(Guard::new(id + 2, log.clone()))) {
		log.note(format!("tag {}", guard.id()));
	}
	tags.entry(2).or_insert(Held::Two(
		Guard::new(id + 3, log.clone()),
		Guard::new(id + 4, log.clone()),
	));
	log.note(format!("tags {}", tags.len()));
	check(4);
	drop(map);
	count("dropped", &name);
	count("dropped", &other);
}

/// A value that logs its drop, and panics after if it blows.
struct Bomb {
	id: u32,
	log: Log,
	blows: bool,
}

impl Drop for Bomb {
	fn drop(&mut self) {
		self.log.note(format!("b{} drop", self.id));
		if self.blows {
			panic!("b{} blew", self.id);
		}
	}
}

/// A value whose own `Drop` may panic, before its fields drop.
struct Shell {
	log: Log,
	blows: bool,
	first: Bomb,
	second: Guard,
}

impl Drop for Shell {
	fn drop(&mut self) {
		self.log.note(format!("shell drop"));
		if self.blows {
			panic!("shell blew");
		}
	}
}

/// What drops after a value's `Drop` panics, scenario `at`'s alone: the rest of the
/// value, and the rest of what holds it, as std's glue and collections go on.
fn blown(log: &Log, id: u32, at: u8) {
	let bomb = |n: u32, blows: bool| Bomb {
		id: id + n,
		log: log.clone(),
		blows,
	};
	let key = |n: u32, fault: Fault| Key::faulty(id + n, log.clone(), fault);
	let guard = |n: u32| Guard::new(id + n, log.clone());
	match at {
		0 => drop(list![bomb(1, false), bomb(2, true), bomb(3, false)]),
		1 => drop(Shell {
			log: log.clone(),
			blows: true,
			first: bomb(1, false),
			second: guard(2),
		}),
		2 => drop(Shell {
			log: log.clone(),
			blows: false,
			first: bomb(1, true),
			second: guard(2),
		}),
		3 => drop((
			guard(1),
			Entries(list![(bomb(2, true), guard(3)), (bomb(4, false), guard(5))]),
			Held::One(guard(6)),
		)),
		4 => {
			let mut map = BTreeMap::new();
			map.insert(key(1, Fault::Sound), bomb(1, false));
			map.insert(key(2, Fault::Sound), bomb(2, true));
			map.insert(key(3, Fault::Sound), bomb(3, false));
			map.clear();
		}
		5 => {
			let mut map = BTreeMap::new();
			map.insert(key(1, Fault::Sound), guard(1));
			map.insert(key(2, Fault::Blows), guard(2));
			map.insert(key(3, Fault::Sound), guard(3));
			drop(map);
		}
		6 => {
			let mut queue = VecDeque::new();
			for n in 1..5 {
				queue.push_back(bomb(n, n == 2));
			}
			queue.push_front(bomb(0, false));
			queue.truncate(1);
		}
		_ => {
			// One entry, whose order std's hash table leaves no room to differ in, and
			// which leaves the map empty as unwinding drops it.
			let mut map = HashMap::new();
			map.insert(key(1, Fault::Blows), guard(1));
			map.clear();
		}
	}
	log.note(format!("unreached"));
}

/// What std drops of what a call owns when a key's `Hash`, `Eq`, or `Ord` panics in it,
/// or a closure it calls does, scenario `at`'s alone, of a B-tree or else a hash table.
fn tripped(log: &Log, id: u32, at: u8, tree: bool) {
	let key = |n: u32| Key::new(id + n, log.clone());
	let trip = |n: u32| Key::faulty(id + n, log.clone(), Fault::Trips);
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let mut map = BTreeMap::new();
	map.insert(key(1), guard(1));
	let mut hashed = HashMap::new();
	hashed.insert(key(2), guard(2));
	match at {
		0 if tree => drop(map.insert(trip(3), guard(3))),
		0 => drop(hashed.insert(trip(3), guard(3))),
		1 if tree => {
			let mut set = BTreeSet::new();
			set.insert(key(3));
			drop(set.insert(trip(4)));
		}
		1 => {
			let mut set = HashSet::new();
			set.insert(key(3));
			drop(set.insert(trip(4)));
		}
		2 if tree => drop(map.entry(trip(3)).or_insert(guard(3))),
		2 => drop(hashed.entry(trip(3)).or_insert(guard(3))),
		3 if tree => drop(map.entry(key(3)).or_insert_with(|| panic!("d{id} made"))),
		3 => drop(
			hashed
				.entry(key(3))
				.or_insert_with_key(|k| panic!("d{id} made {}", k.id)),
		),
		4 if tree => drop(Some(id).map_or(guard(3), |n| -> Guard { panic!("d{n} mapped") })),
		4 => drop(Arc::new(key(3)).max(Arc::new(trip(4)))),
		5 if tree => drop(Err::<u8, Key>(key(3)).unwrap()),
		5 => drop(Err::<u8, Key>(key(3)).expect("an error")),
		6 if tree => drop(Ok::<Key, u8>(key(3)).unwrap_err()),
		6 => drop(Err::<u8, Key>(key(3)).inspect_err(|k| panic!("k{} inspected", k.id))),
		_ if tree => drop(map.remove(&trip(3))),
		_ => drop(hashed.remove(&trip(3))),
	}
	log.note(format!("unreached"));
}

/// What std drops as a map or set grows by a key whose `Hash`, `Eq`, or `Ord` panics, or
/// whose drop does as an equal one is there, scenario `at`'s alone, of a B-tree or else a
/// hash table: into an empty or a cleared one too.
fn grown(log: &Log, id: u32, at: u8, tree: bool) {
	let key = |n: u32| Key::new(id + n, log.clone());
	let fault = |n: u32, fault: Fault| Key::faulty(id + n, log.clone(), fault);
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let mut map = BTreeMap::new();
	let mut hashed = HashMap::new();
	match at {
		0 if tree => drop(map.insert(fault(1, Fault::Trips), guard(1))),
		0 => drop(hashed.insert(fault(1, Fault::Trips), guard(1))),
		1 if tree => {
			map.insert(key(1), guard(1));
			map.clear();
			drop(map.insert(fault(2, Fault::Trips), guard(2)));
		}
		1 => {
			hashed.insert(key(1), guard(1));
			hashed.clear();
			drop(hashed.insert(fault(2, Fault::Trips), guard(2)));
		}
		2 if tree => drop(BTreeSet::new().insert(fault(1, Fault::Trips))),
		2 => drop(HashSet::new().insert(fault(1, Fault::Trips))),
		3 if tree => {
			map.insert(key(1), guard(1));
			drop(map.insert(fault(1, Fault::Blows), guard(2)));
		}
		3 => {
			hashed.insert(key(1), guard(1));
			drop(hashed.insert(fault(1, Fault::Blows), guard(2)));
		}
		4 if tree => {
			map.insert(key(1), guard(1));
			map.clear();
			drop(map.entry(fault(2, Fault::Trips)).or_insert(guard(2)));
		}
		4 => {
			hashed.insert(key(1), guard(1));
			hashed.clear();
			drop(hashed.entry(fault(2, Fault::Trips)).or_insert(guard(2)));
		}
		5 if tree => {
			map.insert(key(1), guard(1));
			drop(map.entry(fault(1, Fault::Blows)).or_insert(guard(2)));
		}
		5 => {
			hashed.insert(key(1), guard(1));
			drop(hashed.entry(fault(1, Fault::Blows)).or_insert(guard(2)));
		}
		6 if tree => {
			let mut set = BTreeSet::new();
			set.insert(key(1));
			drop(set.insert(fault(1, Fault::Blows)));
		}
		6 => {
			let mut set = HashSet::new();
			set.insert(key(1));
			drop(set.insert(fault(1, Fault::Blows)));
		}
		_ if tree => drop(map.entry(fault(1, Fault::Trips)).or_insert(guard(1))),
		_ => drop(hashed.entry(fault(1, Fault::Trips)).or_insert(guard(1))),
	}
	log.note(format!("unreached"));
}

/// What a `Vec`'s edits, its, a tuple's, and a box's `clone`, `Ord`'s defaults, and an
/// iterator's `max` and `min` drop, scenario `at`'s alone, and what is left to drop when a
/// closure, a clone, a comparison, a drop, or an index panics partway.
pub fn edited(log: &Log, id: u32, at: u8) {
	let key = |n: u32| Key::new(id + n, log.clone());
	let fault = |n: u32, fault: Fault| Key::faulty(id + n, log.clone(), fault);
	let ids = |v: &[Key]| v.iter().map(|k| (k.id - id).to_string()).collect::<Vec<_>>().join(",");
	match at {
		0 => {
			let mut v = list![key(1), key(2), key(3), key(4)];
			v.retain(|k| (k.id - id) % 2 == 1);
			log.note(format!("retained {}", ids(&v)));
			v.retain_mut(|k| k.id != id + 3);
			log.note(format!("retained {}", ids(&v)));
			// `max` blows as `clamp` drops it, and `min` still drops.
			drop(key(2).clamp(key(1), fault(3, Fault::Blows)));
		}
		1 => {
			let mut v = list![key(1), key(2), key(3), key(4)];
			v.retain(|k| {
				if k.id == id + 3 {
					panic!("k{} retained", k.id);
				}
				k.id != id + 2
			});
		}
		2 => {
			let mut v = list![key(1), key(1), key(2), key(2), key(3)];
			v.dedup();
			log.note(format!("deduped {}", ids(&v)));
			v.dedup_by_key(|k| k.id / 2);
			log.note(format!("deduped {}", ids(&v)));
			v.dedup_by(|a, b| a.id > b.id);
			log.note(format!("deduped {}", ids(&v)));
			// Bounds out of order, so `clamp`'s assertion panics holding all three.
			drop(key(2).clamp(key(3), key(1)));
		}
		3 => {
			let mut v = list![key(1), fault(1, Fault::Blows), key(1), key(2)];
			v.dedup();
		}
		4 => {
			let mut v = list![key(1), key(2), key(3)];
			drop(v.swap_remove(0));
			log.note(format!("swapped {}", ids(&v)));
			drop(v.swap_remove(3));
		}
		5 => {
			let v = list![key(1), key(2)];
			log.note(format!("cloned {}", ids(&v.clone())));
			let tripped = list![key(1), fault(2, Fault::Trips), key(3)];
			drop(tripped.clone());
		}
		6 => {
			let pair = (key(1), key(2));
			drop(pair.clone());
			let tripped = (key(1), fault(2, Fault::Trips));
			drop(tripped.clone());
		}
		7 => {
			let mut v = list![key(1), key(2), key(3)];
			v.insert(1, key(4));
			log.note(format!("inserted {}", ids(&v)));
			drop(v.remove(0));
			log.note(format!("removed {}", ids(&v)));
			v.truncate(1);
			log.note(format!("truncated {}", ids(&v)));
			v.clear();
			// Past the end, the key it was given drops as it panics.
			v.insert(1, key(5));
		}
		8 => {
			// Each loser drops as it loses.
			let best = list![key(1), key(3), key(5)].into_iter().max_by_key(|k| (k.id - id) % 3);
			log.note(format!("max {}", best.map_or(0, |k| k.id - id)));
			let least = list![key(3), key(1), key(2)].into_iter().min();
			log.note(format!("min {}", least.map_or(0, |k| k.id - id)));
			// A comparison that panics drops the item, the best, and then the rest.
			drop(list![key(1), key(2), fault(3, Fault::Trips), key(4)].into_iter().max());
		}
		9 => {
			let boxed = Box::new(key(1));
			log.note(format!("boxed {}", boxed.clone().id - id));
			// A drop that panics still drops the rest.
			let mut v = list![key(2), fault(3, Fault::Blows), key(4)];
			v.clear();
		}
		_ => {
			log.note(format!("max {}", key(1).max(key(2)).id - id));
			log.note(format!("min {}", key(5).min(key(4)).id - id));
			log.note(format!("clamped {}", key(2).clamp(key(1), key(3)).id - id));
			log.note(format!("clamped {}", key(0).clamp(key(1), key(3)).id - id));
			// A comparison that panics in `max`, holding both.
			drop(key(1).max(fault(2, Fault::Trips)));
		}
	}
	log.note(format!("unreached"));
}
