//! What a closure drops of the values it owns: where it goes out of scope, where std
//! discards it uncalled, where a call consumes it, and where std's iterators and
//! collections are done with it, a panic included.

use std::collections::{BTreeMap, VecDeque};

use super::Bomb;
use crate::log::Log;
use crate::model::Guard;

/// Call `f` once, which consumes it.
fn once<R>(f: impl FnOnce() -> R) -> R {
	f()
}

/// Call `f` with each of `n` and `n + 1`, which borrows it, and drop it.
fn twice(n: u32, mut f: impl FnMut(u32) -> u32) -> u32 {
	f(n) + f(n + 1)
}

/// The id of `bomb`, which a closure that calls it captures whole, where one that reads
/// its field captures only that.
fn bomb_id(bomb: &Bomb) -> u32 {
	bomb.id
}

/// Hold `f`, which drops with it.
struct Holder<F> {
	f: F,
	guard: Guard,
}

/// What closures that own guards drop, panicking at point `at`, if any, some inside a
/// closure's body.
pub(super) fn scoped(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	// Out of scope uncalled, or called by reference and then dropped, as the scope ends or
	// a panic unwinds it.
	let (a, b) = (guard(1), guard(2));
	let unused = move || a.id() + b.id();
	let c = guard(3);
	let read = move |n: u32| n + c.id();
	log.note(format!("read {}", read(1) + read(2)));
	check(1);
	drop(unused);
	// A body that moves what it owns out drops it there; one that keeps it drops it as it
	// returns, or as a panic in it unwinds.
	let d = guard(4);
	let moved = once(move || {
		let n = d.id();
		drop(d);
		log.note(format!("moved {n}"));
		n
	});
	log.note(format!("moved {moved}"));
	let e = guard(5);
	let kept = once(move || {
		check(2);
		e.id()
	});
	log.note(format!("kept {kept}"));
	// One that `Fn` calls, consumed by a call through `FnOnce`, drops after it, or as a
	// panic in it unwinds.
	let f = guard(6);
	let by_ref = move || {
		check(3);
		f.id()
	};
	log.note(format!("by ref {}", once(by_ref)));
	let g = guard(7);
	log.note(format!("twice {}", twice(1, move |n| n + g.id())));
	// Held in a struct, and in a closure that owns it.
	let h = guard(8);
	let holder = Holder {
		f: move |n: u32| n + h.id(),
		guard: guard(9),
	};
	log.note(format!("holder {}", (holder.f)(1) + holder.guard.id()));
	let i = guard(10);
	let inner = move |n: u32| n + i.id();
	let outer = move || inner(1);
	check(4);
	log.note(format!("outer {}", once(outer)));
	drop(holder);
	// Changing what it owns, through `FnMut`.
	let mut held = Vec::new();
	let mut push = move |g: Guard| {
		held.push(g);
		held.len()
	};
	push(guard(11));
	push(guard(12));
	check(5);
	drop(push);
}

/// What std does with a closure that owns a guard: drops it where it is not called, and
/// leaves it to the call where it is.
pub(super) fn discarded(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	let some = if at < 8 { Some(id) } else { None };
	let none = if at > 8 { Some(id) } else { None };
	let (a, b) = (guard(1), guard(2));
	let mapped = some.map(move |n| {
		check(1);
		n + a.id()
	});
	let unmapped = none.map(move |n| n + b.id());
	log.note(format!("map {mapped:?} {unmapped:?}"));
	let ok = Ok::<u32, u32>(id);
	let err = Err::<u32, u32>(id);
	let (c, d, e, f) = (guard(3), guard(4), guard(5), guard(6));
	let values = (ok.map(move |n| n + c.id()), err.map(move |n| n + d.id()));
	let errors = (
		ok.map_err(move |n| n + e.id()),
		err.map_err(move |n| {
			check(2);
			n + f.id()
		}),
	);
	log.note(format!("result {values:?} {errors:?}"));
	let (g, h) = (guard(7), guard(8));
	let chained = (
		ok.and_then(move |n| Ok::<u32, u32>(n + g.id())),
		err.and_then(move |n| Ok::<u32, u32>(n + h.id())),
	);
	let (i, j) = (guard(9), guard(10));
	let inspected = (
		ok.inspect_err(move |_| log.note(format!("inspect {}", i.id()))),
		err.inspect_err(move |_| {
			check(3);
			log.note(format!("inspect {}", j.id()))
		}),
	);
	log.note(format!("chained {chained:?} {inspected:?}"));
	let (k, l) = (guard(11), guard(12));
	let defaults = (
		some.map_or(0, move |n| {
			check(4);
			n + k.id()
		}),
		none.map_or(0, move |n| n + l.id()),
	);
	log.note(format!("map_or {defaults:?}"));
	let (m, n) = (guard(13), guard(14));
	let yes = some.is_some();
	let thens = (yes.then(move || m.id()), (!yes).then(move || n.id()));
	log.note(format!("then {thens:?}"));
	// One that `Fn` calls, which std consumes by `FnOnce`: called, then dropped.
	let (o, p) = (guard(15), guard(16));
	let plus = move |n: u32| {
		check(5);
		n + o.id()
	};
	let other = move |n: u32| n + p.id();
	log.note(format!("by ref {:?} {:?}", some.map(plus), none.map(other)));
	let (r, s) = (guard(18), guard(19));
	let kept = move || r.id();
	let skipped = move || s.id();
	log.note(format!("then by ref {:?} {:?}", yes.then(kept), (!yes).then(skipped)));
	// A closure that owns one that owns a guard, as a helper's would.
	let q = guard(17);
	let inner = move |n: u32| n + q.id();
	log.note(format!(
		"nested {:?}",
		some.map(move |n| {
			check(6);
			inner(n)
		})
	));
	check(7);
}

/// What std's iterators and collections drop of a closure that owns a guard: an adaptor
/// after the iterator it wraps, and a call that keeps nothing of it as it returns, with
/// what each drops of its own before it, panicking at point `at`, if any, some inside a
/// closure.
pub(super) fn iterated(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	// Adaptors, read in part and dropped, or ended by a `for`.
	let a = guard(1);
	let mut mapped = list![guard(2), guard(3), guard(4)].into_iter().map(move |g| {
		if g.id() == id + 3 {
			check(1);
		}
		g.id() + a.id()
	});
	log.note(format!("map {:?} {:?}", mapped.next(), mapped.next()));
	drop(mapped);
	let b = guard(5);
	for g in list![guard(6), guard(7), guard(8)]
		.into_iter()
		.filter(move |g| g.id() != b.id() + 1)
	{
		log.note(format!("filter {}", g.id()));
		if g.id() == id + 7 {
			break;
		}
	}
	let (c, d, e) = (guard(9), guard(10), guard(11));
	let mut chained = list![guard(12), guard(13), guard(14), guard(15)]
		.into_iter()
		.skip_while(move |g| g.id() < c.id() + 4)
		.take_while(move |g| g.id() < d.id() + 5)
		.inspect(move |g| {
			if g.id() == id + 14 {
				check(2);
			}
			log.note(format!("inspect {} {}", g.id(), e.id()));
		});
	log.note(format!("chained {:?}", chained.next().map(|g| g.id())));
	log.note(format!("chained {:?}", chained.next().map(|g| g.id())));
	drop(chained);
	let (f, g) = (guard(16), guard(17));
	let mut flat = list![1u32, 2, 3]
		.into_iter()
		.filter_map(move |n| (n != 2).then_some(n + f.id()))
		.flat_map(move |n| list![n, n + g.id()]);
	log.note(format!("flat {:?}", flat.next()));
	check(3);
	drop(flat);
	let h = guard(18);
	let whiles: Vec<u32> = [1u32, 2, 3]
		.iter()
		.map_while(move |&n| (n < 3).then_some(n + h.id()))
		.collect();
	log.note(format!("map_while {whiles:?}"));
	// Lazy sources, which hold their closure.
	let (i, j) = (guard(19), guard(20));
	let made = std::iter::from_fn(move || {
		check(4);
		Some(i.id())
	})
	.take(2)
	.count();
	let walked: u32 = std::iter::successors(Some(1u32), move |&n| (n < 3).then_some(n + 1 + j.id() - j.id())).sum();
	log.note(format!("lazy {made} {walked}"));
	// Consumers that borrow the iterator, which drop the closure as they return.
	let items = list![guard(21), guard(22), guard(23)];
	let (k, l, m) = (guard(24), guard(25), guard(26));
	let any = items.iter().any(move |g| {
		if g.id() == id + 22 {
			check(5);
		}
		g.id() == k.id()
	});
	let found = items.iter().find(move |g| g.id() == l.id() - 3).map(|g| g.id());
	let at_pos = items.iter().position(move |g| g.id() + 2 == m.id());
	log.note(format!("found {any} {found:?} {at_pos:?}"));
	// One that drops nothing else by value, folded.
	let n = guard(27);
	let sum: u32 = [1u32, 2, 3].iter().map(move |&x| x + n.id()).sum();
	let o = guard(28);
	[1u32, 2]
		.iter()
		.for_each(move |&x| log.note(format!("each {}", x + o.id())));
	log.note(format!("sum {sum}"));
	// Items made as it goes, which it drops but for the best, before its key's closure.
	let r = guard(36);
	let best = [1u32, 2, 3].iter().map(|&n| guard(36 + n)).max_by_key(move |g| {
		if g.id() == id + 38 {
			check(7);
		}
		g.id() % 2 + r.id()
	});
	log.note(format!("best {:?}", best.map(|g| g.id())));
	// What `retain` drops of its own before the closure.
	let mut map = BTreeMap::new();
	for (k, n) in [(1u32, 29), (2, 30), (3, 31)] {
		map.insert(k, guard(n));
	}
	let p = guard(32);
	map.retain(move |&k, _| {
		if k == 2 {
			check(6);
		}
		k + p.id() != id + 34
	});
	let mut queue = VecDeque::new();
	queue.push_back(guard(33));
	queue.push_back(guard(34));
	let q = guard(35);
	queue.retain(move |g| g.id() + 2 != q.id());
	log.note(format!("retained {} {}", map.len(), queue.len()));
}

/// What drops of a closure's captures when one's `Drop` panics: the rest still drop, in
/// order, and the panic goes on.
pub(super) fn blasted(log: &Log, id: u32, at: u8) {
	let bomb = |n: u32, blows: bool| Bomb {
		id: id + n,
		log: log.clone(),
		blows,
	};
	let guard = |n: u32| Guard::new(id + n, log.clone());
	match at {
		0 => {
			let (a, b, c) = (bomb(1, false), bomb(2, true), guard(3));
			let f = move || bomb_id(&a) + bomb_id(&b) + c.id();
			log.note(format!("blasted {}", f()));
		}
		1 => {
			let (a, b) = (bomb(1, true), guard(2));
			drop(None::<u32>.map(move |n| n + bomb_id(&a) + b.id()));
		}
		2 => {
			let (a, b) = (bomb(1, true), guard(2));
			drop(once(move || bomb_id(&a) + b.id()));
		}
		3 => {
			let (a, b) = (bomb(1, true), guard(2));
			let f = move || bomb_id(&a) + b.id();
			drop(once(f));
		}
		_ => {
			let (a, b) = (bomb(1, true), guard(2));
			let f = move || bomb_id(&a) + b.id();
			let g = move || f();
			let h = guard(3);
			drop(g);
			drop(h);
		}
	}
}
