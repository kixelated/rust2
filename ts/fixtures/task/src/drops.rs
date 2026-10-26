//! Values that drop inside what std holds them in: an enum's, which has no
//! `[Symbol.dispose]` of its own, in each std type that drops what it holds, and generic
//! types that implement `Drop`. Each step logs every drop, so the log shows the order,
//! and a step that panics shows what unwinding dropped.

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

/// Run scenario `arg % 4`, panicking at the point `arg / 4 % 8` names, if any: what
/// every value std holds drops, where Rust drops it, and in Rust's order.
pub fn run(log: &Log, id: u32, arg: u8) {
	let at = arg / 4 % 8;
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	match arg % 4 {
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
