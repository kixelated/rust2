//! What a closure drops of the values it owns: where it goes out of scope, where std
//! discards it uncalled, where a call consumes it, and where std's iterators and
//! collections are done with it.

use std::collections::{BTreeMap, VecDeque};

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

/// Hold `f`, which drops with it.
struct Holder<F> {
	f: F,
	guard: Guard,
}

/// What closures that own guards drop.
pub(super) fn scoped(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	// Out of scope uncalled, or called by reference and then dropped, as the scope ends.
	let (a, b) = (guard(1), guard(2));
	let unused = move || a.id() + b.id();
	let c = guard(3);
	let read = move |n: u32| n + c.id();
	log.note(format!("read {}", read(1) + read(2)));
	drop(unused);
	// A body that moves what it owns out drops it there; one that keeps it drops it as it
	// returns.
	let d = guard(4);
	let moved = once(move || {
		let n = d.id();
		drop(d);
		log.note(format!("moved {n}"));
		n
	});
	log.note(format!("moved {moved}"));
	let e = guard(5);
	let kept = once(move || e.id());
	log.note(format!("kept {kept}"));
	// One that `Fn` calls, consumed by a call through `FnOnce`, drops after it.
	let f = guard(6);
	let by_ref = move || f.id();
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
	drop(push);
}

/// What std does with a closure that owns a guard: drops it where it is not called, and
/// leaves it to the call where it is.
pub(super) fn discarded(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let some = Some(id);
	let none = None::<u32>;
	let (a, b) = (guard(1), guard(2));
	let mapped = some.map(move |n| n + a.id());
	let unmapped = none.map(move |n| n + b.id());
	log.note(format!("map {mapped:?} {unmapped:?}"));
	let ok = Ok::<u32, u32>(id);
	let err = Err::<u32, u32>(id);
	let (c, d, e, f) = (guard(3), guard(4), guard(5), guard(6));
	let values = (ok.map(move |n| n + c.id()), err.map(move |n| n + d.id()));
	let errors = (ok.map_err(move |n| n + e.id()), err.map_err(move |n| n + f.id()));
	log.note(format!("result {values:?} {errors:?}"));
	let (g, h) = (guard(7), guard(8));
	let chained = (
		ok.and_then(move |n| Ok::<u32, u32>(n + g.id())),
		err.and_then(move |n| Ok::<u32, u32>(n + h.id())),
	);
	let (i, j) = (guard(9), guard(10));
	let inspected = (
		ok.inspect_err(move |_| log.note(format!("inspect {}", i.id()))),
		err.inspect_err(move |_| log.note(format!("inspect {}", j.id()))),
	);
	log.note(format!("chained {chained:?} {inspected:?}"));
	let (k, l) = (guard(11), guard(12));
	let defaults = (some.map_or(0, move |n| n + k.id()), none.map_or(0, move |n| n + l.id()));
	log.note(format!("map_or {defaults:?}"));
	let (m, n) = (guard(13), guard(14));
	let yes = some.is_some();
	let thens = (yes.then(move || m.id()), (!yes).then(move || n.id()));
	log.note(format!("then {thens:?}"));
	// One that `Fn` calls, which std consumes by `FnOnce`: called, then dropped.
	let (o, p) = (guard(15), guard(16));
	let plus = move |n: u32| n + o.id();
	let other = move |n: u32| n + p.id();
	log.note(format!("by ref {:?} {:?}", some.map(plus), none.map(other)));
	let (r, s) = (guard(18), guard(19));
	let kept = move || r.id();
	let skipped = move || s.id();
	log.note(format!("then by ref {:?} {:?}", yes.then(kept), (!yes).then(skipped)));
	// A closure that owns one that owns a guard, as a helper's would.
	let q = guard(17);
	let inner = move |n: u32| n + q.id();
	log.note(format!("nested {:?}", some.map(move |n| inner(n))));
}

/// What std's iterators and collections drop of a closure that owns a guard: an adaptor
/// after the iterator it wraps, and a call that keeps nothing of it as it returns, with
/// what each drops of its own before it.
pub(super) fn iterated(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	// Adaptors, read in part and dropped, or ended by a `for`.
	let a = guard(1);
	let mut mapped = list![guard(2), guard(3), guard(4)]
		.into_iter()
		.map(move |g| g.id() + a.id());
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
	drop(flat);
	// Ended, it keeps its closure, since the source it maps is fused.
	let w = guard(47);
	let mut ended = [1u32].iter().flat_map(move |&n| [n, w.id()]);
	while ended.next().is_some() {}
	log.note(format!("flat_map ended"));
	drop(ended);
	let h = guard(18);
	let whiles: Vec<u32> = [1u32, 2, 3]
		.iter()
		.map_while(move |&n| (n < 3).then_some(n + h.id()))
		.collect();
	log.note(format!("map_while {whiles:?}"));
	// Lazy sources, which hold their closure.
	let (i, j) = (guard(19), guard(20));
	let made = std::iter::from_fn(move || Some(i.id())).take(2).count();
	let walked: u32 = std::iter::successors(Some(1u32), move |&n| (n < 3).then_some(n + 1 + j.id() - j.id())).sum();
	// One of guards, which holds the next, made by a closure that owns one, and drops the
	// one it holds before its closure.
	let s = guard(43);
	let mut chain = std::iter::successors(Some(guard(40)), move |g| {
		(g.id() + 1 < s.id()).then(|| Guard::new(g.id() + 1, log.clone()))
	});
	log.note(format!("lazy {made} {walked} {:?}", chain.next().map(|g| g.id())));
	log.note(format!("successor {:?}", chain.next().map(|g| g.id())));
	drop(chain);
	// Consumers that borrow the iterator, which drop the closure as they return.
	let items = list![guard(21), guard(22), guard(23)];
	let (k, l, m) = (guard(24), guard(25), guard(26));
	let any = items.iter().any(move |g| g.id() == k.id());
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
	// Consumers that count or end an adaptor through what it wraps, which drop its closure
	// once.
	let (t, u, v) = (guard(44), guard(45), guard(46));
	let peeked = [1u32, 2].iter().map(move |&x| x + t.id()).peekable().count();
	let skipped = [1u32, 2, 3].iter().map(move |&x| x + u.id()).skip(1).count();
	let last = [1u32, 2].iter().map(move |&x| x + v.id()).skip(1).last();
	log.note(format!("counted {peeked} {skipped} {last:?}"));
	// Items made as it goes, which it drops but for the best, before its key's closure.
	let r = guard(36);
	let best = [1u32, 2, 3]
		.iter()
		.map(|&n| guard(36 + n))
		.max_by_key(move |g| g.id() % 2 + r.id());
	log.note(format!("best {:?}", best.map(|g| g.id())));
	// What `retain` drops of its own before the closure.
	let mut map = BTreeMap::new();
	for (k, n) in [(1u32, 29), (2, 30), (3, 31)] {
		map.insert(k, guard(n));
	}
	let p = guard(32);
	map.retain(move |&k, _| k + p.id() != id + 34);
	let mut queue = VecDeque::new();
	queue.push_back(guard(33));
	queue.push_back(guard(34));
	let q = guard(35);
	queue.retain(move |g| g.id() + 2 != q.id());
	log.note(format!("retained {} {}", map.len(), queue.len()));
	// An array's `map` moves each element into its closure, which drops after the last call,
	// and its result holds what the closure kept until it drops.
	let x = guard(48);
	let kept = [guard(49), guard(50), guard(51)].map(move |g| {
		log.note(format!("array map {}", g.id()));
		(g.id() != x.id() + 2).then_some(g)
	});
	log.note(format!(
		"array mapped {:?}",
		kept.each_ref().map(|g| g.as_ref().map(Guard::id))
	));
}
