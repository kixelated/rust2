//! What std's by-value consumers drop of an iterator whose adaptors hold closures that
//! drop beside items that drop: `fold` drops each adaptor's closure as its own fold ends,
//! before the source it wraps, where dropping the adaptor drops it after. Each closure
//! panics as it reads the item `at` names, if any, to show what unwinding drops.

use std::collections::{BTreeMap, VecDeque};

use crate::log::Log;
use crate::model::Guard;

/// Run scenario `arg % 16`, panicking as a closure reads item `arg / 16`, if any.
pub(super) fn folded(log: &Log, id: u32, arg: u8) {
	let at = id + u32::from(arg / 16);
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let guards = |from: u32, to: u32| (from..to).map(guard).collect::<Vec<_>>();
	let trip = move |g: &Guard| {
		if g.id() == at {
			panic!("d{id} at {}", g.id() - id);
		}
	};
	let trips = move |n: u32| {
		if id + n == at {
			panic!("d{id} at {n}");
		}
	};
	match arg % 16 {
		0 => {
			let c = guard(10);
			let n = guards(1, 5)
				.into_iter()
				.map(move |g| {
					trip(&g);
					g.id() + c.id()
				})
				.count();
			log.note(format!("count {n}"));
			let d = guard(11);
			let sum: u32 = guards(5, 8)
				.into_iter()
				.map(move |g| {
					trip(&g);
					g.id() - id + d.id() - id
				})
				.sum();
			log.note(format!("sum {sum}"));
		}
		1 => {
			let c = guard(10);
			let n = guards(1, 7)
				.into_iter()
				.filter(move |g| {
					trip(g);
					g.id() % 2 == c.id() % 2
				})
				.fold(0, |n, g| {
					log.note(format!("fold {}", g.id()));
					n + 1
				});
			log.note(format!("fold {n}"));
		}
		2 => {
			let c = guard(10);
			guards(1, 7)
				.into_iter()
				.filter_map(move |g| {
					trip(&g);
					(g.id() % 3 != c.id() % 3).then_some(g)
				})
				.for_each(|g| log.note(format!("each {}", g.id())));
		}
		3 => {
			let c = guard(10);
			let last = guards(1, 5)
				.into_iter()
				.inspect(move |g| {
					trip(g);
					log.note(format!("inspect {} {}", g.id(), c.id()));
				})
				.last();
			log.note(format!("last {:?}", last.map(|g| g.id())));
		}
		4 => {
			let c = guard(10);
			let best = guards(1, 6)
				.into_iter()
				.map(move |g| {
					trip(&g);
					log.note(format!("map {} {}", g.id(), c.id()));
					g
				})
				.max_by_key(|g| g.id() % 3);
			log.note(format!("best {:?}", best.map(|g| g.id())));
			let d = guard(11);
			let max = guards(6, 9)
				.into_iter()
				.map(move |g| {
					trip(&g);
					g.id() % d.id()
				})
				.max();
			log.note(format!("max {max:?}"));
		}
		5 => {
			// In place of the `Vec` it reads, and not.
			let c = guard(10);
			let same: Vec<Guard> = guards(1, 4)
				.into_iter()
				.map(move |g| {
					trip(&g);
					log.note(format!("map {} {}", g.id(), c.id()));
					g
				})
				.collect();
			let d = guard(11);
			let wider: Vec<(Guard, u64)> = guards(4, 7)
				.into_iter()
				.map(move |g| {
					trip(&g);
					(g, u64::from(d.id()))
				})
				.collect();
			log.note(format!("collected {} {}", same.len(), wider.len()));
		}
		6 => {
			let c = guard(10);
			let queue: VecDeque<Guard> = guards(1, 4)
				.into_iter()
				.map(move |g| {
					trip(&g);
					log.note(format!("map {} {}", g.id(), c.id()));
					g
				})
				.collect();
			let d = guard(11);
			let tree: BTreeMap<u32, Guard> = guards(4, 7)
				.into_iter()
				.map(move |g| {
					trip(&g);
					(g.id() + d.id(), g)
				})
				.collect();
			let e = guard(12);
			let mut v = guards(7, 8);
			v.extend(guards(8, 10).into_iter().map(move |g| {
				trip(&g);
				log.note(format!("map {} {}", g.id(), e.id()));
				g
			}));
			log.note(format!("collected {} {} {}", queue.len(), tree.len(), v.len()));
		}
		7 => {
			// Two closures that drop, and nothing else.
			let (c, d) = (guard(10), guard(11));
			let n = [1u32, 2, 3, 4, 5]
				.iter()
				.map(move |&n| {
					trips(n);
					n + c.id()
				})
				.filter(move |n| {
					trips(*n - id - 4);
					n % 2 == d.id() % 2
				})
				.count();
			log.note(format!("count {n}"));
		}
		8 => {
			// Through a chain, which folds each half in turn.
			let c = guard(10);
			let n: u32 = guards(1, 3)
				.into_iter()
				.chain(guards(3, 5))
				.map(move |g| {
					trip(&g);
					g.id() - id + c.id() - id
				})
				.sum();
			let d = guard(11);
			let m = guards(5, 7)
				.into_iter()
				.map(move |g| {
					trip(&g);
					g.id() + d.id()
				})
				.chain([1u32, 2])
				.count();
			let e = guard(12);
			let k = guards(7, 9)
				.into_iter()
				.chain(guards(9, 11).into_iter().filter(move |g| {
					trip(g);
					g.id() != e.id()
				}))
				.fold(0, |n, g| n + g.id());
			log.note(format!("chain {n} {m} {k}"));
		}
		9 => {
			// Through what reads past items or reorders them.
			let c = guard(10);
			let n = guards(1, 7)
				.into_iter()
				.map(move |g| {
					trip(&g);
					log.note(format!("map {} {}", g.id(), c.id()));
					g
				})
				.rev()
				.skip(1)
				.step_by(2)
				.take(2)
				.fold(0, |n, g| n + g.id() - id);
			let d = guard(11);
			let m: usize = guards(7, 10)
				.into_iter()
				.enumerate()
				.map(move |(i, g)| {
					trip(&g);
					i + d.id() as usize
				})
				.sum();
			log.note(format!("read {n} {m}"));
		}
		10 => {
			// A peeked item, and an ended iterator's fuse.
			let c = guard(10);
			let mut peek = guards(1, 5)
				.into_iter()
				.map(move |g| {
					trip(&g);
					log.note(format!("map {} {}", g.id(), c.id()));
					g
				})
				.peekable();
			log.note(format!("peek {:?}", peek.peek().map(|g| g.id())));
			log.note(format!("count {}", peek.count()));
			let d = guard(11);
			let n = guards(5, 8)
				.into_iter()
				.fuse()
				.filter(move |g| {
					trip(g);
					g.id() != d.id()
				})
				.fold(0, |n, g| n + g.id());
			log.note(format!("fuse {n}"));
		}
		11 => {
			// Closures that end the fold early, and ones that skip.
			let c = guard(10);
			let n: u32 = guards(1, 6)
				.into_iter()
				.skip_while(move |g| {
					trip(g);
					g.id() < c.id() - 8
				})
				.take_while(|g| g.id() < id + 5)
				.map(|g| g.id() - id)
				.sum();
			let d = guard(11);
			let m: u32 = guards(6, 10)
				.into_iter()
				.map_while(move |g| {
					trip(&g);
					(g.id() != d.id() - 3).then(|| g.id() - id)
				})
				.sum();
			let e = guard(12);
			let k = guards(10, 13)
				.into_iter()
				.take_while(move |g| {
					trip(g);
					g.id() != e.id() - 1
				})
				.count();
			log.note(format!("while {n} {m} {k}"));
		}
		12 => {
			// What a `flat_map` folds: its closure, and each iterator it makes.
			let c = guard(10);
			let n = [1u32, 3, 5]
				.into_iter()
				.flat_map(move |n| {
					trips(n);
					let v = list![guard(n), guard(n + 1)];
					log.note(format!("flat {n} {}", c.id()));
					v
				})
				.count();
			let d = guard(11);
			let m: u32 = guards(7, 10)
				.into_iter()
				.map(move |g| {
					trip(&g);
					list![g, guard(d.id() - id + 1)]
				})
				.flatten()
				.map(|g| g.id() - id)
				.sum();
			log.note(format!("flat {n} {m}"));
		}
		13 => {
			// Lazy sources, which hold their closure and what it made.
			let mut held = guards(1, 5);
			let n = std::iter::from_fn(move || {
				let g = held.pop()?;
				trip(&g);
				Some(g)
			})
			.map(|g| g.id())
			.count();
			let d = guard(11);
			let m = std::iter::successors(Some(guard(5)), move |g| {
				trip(g);
				(g.id() < id + 8).then(|| Guard::new(g.id() + 1 + d.id() - d.id(), log.clone()))
			})
			.fold(0, |n, g| n + g.id() - id);
			log.note(format!("lazy {n} {m}"));
		}
		14 => {
			// A consumer over the adaptor that holds the closure.
			let c = guard(10);
			let n = guards(1, 5)
				.into_iter()
				.map(move |g| {
					trip(&g);
					g.id() + c.id()
				})
				.skip(1)
				.count();
			let d = guard(11);
			let last = guards(5, 8)
				.into_iter()
				.map(move |g| {
					trip(&g);
					log.note(format!("map {} {}", g.id(), d.id()));
					g
				})
				.enumerate()
				.last();
			log.note(format!("over {n} {:?}", last.map(|(i, g)| (i, g.id()))));
		}
		_ => {
			// Back to front.
			let c = guard(10);
			let n = guards(1, 5)
				.into_iter()
				.map(move |g| {
					trip(&g);
					log.note(format!("map {} {}", g.id(), c.id()));
					g.id() - id
				})
				.rev()
				.fold(0, |n, k| n * 10 + k);
			let d = guard(11);
			let m = guards(5, 7)
				.into_iter()
				.chain(guards(7, 9))
				.filter(move |g| {
					trip(g);
					g.id() != d.id() - 5
				})
				.rev()
				.count();
			log.note(format!("rev {n} {m}"));
		}
	}
}
