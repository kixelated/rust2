//! What the harness spawns and drives: a consumer loop holding guards across its awaits,
//! the same loop under a stack of async functions, and the channels they read.

use crate::kio::{Consumer, Producer};
use crate::log::Log;

/// A channel's open producers, and a consumer to clone for each task.
pub struct Channel {
	producers: Vec<Producer>,
	consumer: Consumer,
}

impl Channel {
	pub fn open() -> Self {
		let producer = Producer::new();
		let consumer = producer.consume();
		let mut producers = Vec::new();
		producers.push(producer);
		Self { producers, consumer }
	}

	/// A consumer that reads every value from the first.
	pub fn consumer(&self) -> Consumer {
		self.consumer.clone()
	}

	/// Send `value` through the first producer, if one is open.
	pub fn send(&self, value: u64) -> bool {
		match self.producers.first() {
			Some(producer) => {
				producer.send(value);
				true
			}
			None => false,
		}
	}

	/// Drop the last producer, if one is open: the last closes the channel.
	pub fn close(&mut self) -> bool {
		match self.producers.pop() {
			Some(producer) => {
				drop(producer);
				true
			}
			None => false,
		}
	}

	/// Open another producer, if one is open to clone.
	pub fn clone_producer(&mut self) -> bool {
		let clone = match self.producers.first() {
			Some(producer) => producer.clone(),
			None => return false,
		};
		self.producers.push(clone);
		true
	}
}

/// Logs its own drop.
pub struct Guard {
	id: u32,
	log: Log,
}

impl Guard {
	pub fn new(id: u32, log: Log) -> Self {
		Self { id, log }
	}

	/// Log `event` as this guard's.
	fn note(&self, event: &str) {
		self.log.note(format!("g{} {event}", self.id));
	}
}

impl Drop for Guard {
	fn drop(&mut self) {
		self.log.note(format!("g{} drop", self.id));
	}
}

/// Every value until the channel closes, summed, holding a guard of its own across
/// each await beside the one it was given. It panics on 99.
pub async fn sum(mut rx: Consumer, guard: Guard) -> u64 {
	let inner = Guard::new(guard.id + 100, guard.log.clone());
	let mut total = 0;
	while let Some(value) = rx.recv().await {
		// A poisoned value panics with every guard live, which unwinding drops.
		assert!(value != 99, "g{} poisoned", guard.id);
		inner.log.note(format!("g{} got {value}", guard.id));
		total += value;
	}
	total
}

/// One value read through the captures only an async block makes: a borrow, which
/// dropping the block must not drop; a copy, which a later change to the original must
/// not reach; a mutable borrow the block replaces whole, which the caller must see; a
/// move on one path only; and a move the block only borrows, which drops when the block
/// returns or is cancelled. Between them, a value of two types that hold each other.
pub async fn blocks(mut rx: Consumer, guard: Guard, moves: bool) -> u64 {
	let first = Guard::new(guard.id + 200, guard.log.clone());
	let second = Guard::new(guard.id + 300, guard.log.clone());
	drop(async { first.note("borrowed") });
	turns(&guard.log, guard.id % 5);
	let mut tally = Tally { n: guard.id };
	let seen = async move { read(tally) };
	tally.n += 1000;
	guard.note(&format!("tally {} then {}", tally.n, seen.await));
	let mut mark = Mark { n: 1 };
	let replaced = async {
		mark = Mark { n: 2 };
		mark.n
	}
	.await;
	guard.note(&format!("mark {replaced} {}", mark.n));
	let cycle = Outer {
		inner: None,
		guard: Guard::new(guard.id + 800, guard.log.clone()),
	};
	let _nested = Inner {
		outer: Some(Box::new(cycle)),
	};
	// A move on each arm of a `match` simple enough to be an expression.
	let third = Guard::new(guard.id + 400, guard.log.clone());
	let chosen = match moves {
		true => third,
		false => third,
	};
	chosen.note("chosen");
	if moves {
		drop(async move { guard.note("moved") });
	}
	let read = async move {
		first.note("reads");
		second.note("reads");
		let _held = Guard::new(second.id + 100, second.log.clone());
		match rx.recv().await {
			Some(value) => value,
			None => 0,
		}
	};
	read.await
}

struct Mark {
	n: u32,
}

/// Drops through `guard`, and `Inner` through it: deciding `Outer` first must not leave
/// `Inner` decided as dropping nothing.
struct Outer {
	inner: Option<Box<Inner>>,
	guard: Guard,
}

struct Inner {
	outer: Option<Box<Outer>>,
}

/// A `Copy` value an `async move` block takes its own copy of.
#[derive(Clone, Copy)]
struct Tally {
	n: u32,
}

fn read(t: Tally) -> u32 {
	t.n
}

/// `i`, noted as picked.
fn pick(log: &Log, i: usize) -> usize {
	log.note(format!("pick {i}"));
	i
}

/// A guard per turn of a loop that ends a guarded `match` arm, so leaving the loop from
/// inside a turn's scope must still skip the arms after it. Before them, a pair of
/// guards destructured in part: the one left in the array drops with it, after the other;
/// a pinned box of one, which drops it; `ref` bindings, which borrow a variable and keep a
/// temporary to the end of the block; a `_` binding of an element, which evaluates its
/// index and moves nothing; and a boxed guard read twice, which a read leaves boxed.
#[expect(clippy::toplevel_ref_arg, reason = "the `ref` bindings are under test")]
fn turns(log: &Log, count: u32) {
	let pair = [Guard::new(count + 10, log.clone()), Guard::new(count + 20, log.clone())];
	let [kept, _] = pair;
	kept.note("kept");
	let pinned = std::pin::Pin::new(Box::new(Guard::new(count + 40, log.clone())));
	pinned.note("pinned");
	let held = Guard::new(count + 50, log.clone());
	let ref borrowed = held;
	borrowed.note("borrowed");
	let ref extended = Guard::new(count + 60, log.clone());
	extended.note("extended");
	let scoped = {
		let ref inner = Guard::new(count + 90, log.clone());
		inner.id
	};
	log.note(format!("after {scoped}"));
	let boxed = Box::new(Guard::new(count + 100, log.clone()));
	let once = boxed.id;
	log.note(format!("boxed {once} {}", boxed.id));
	let spares = [Guard::new(count + 70, log.clone()), Guard::new(count + 80, log.clone())];
	let _ = spares[pick(log, count as usize % 2)];
	match count {
		0 => log.note(format!("no turns")),
		n if n < 4 => {
			let mut left = n;
			loop {
				let turn = Guard::new(left, log.clone());
				if left == 1 {
					break;
				}
				turn.note("turn");
				left -= 1;
			}
		}
		_ => log.note(format!("many turns")),
	}
}

/// Every value until the channel closes, summed, each read `depth` async functions
/// deep: 1, 4, or 16.
pub async fn drain(mut rx: Consumer, depth: u32) -> u64 {
	let mut total = 0;
	loop {
		let next = match depth {
			1 => rx.recv().await,
			4 => level3(&mut rx).await,
			_ => level15(&mut rx).await,
		};
		match next {
			Some(value) => total += value,
			None => break,
		}
	}
	total
}

/// `levelN` reads through N async functions over `recv`.
macro_rules! levels {
	($($name:ident => $next:ident,)*) => {
		$(async fn $name(rx: &mut Consumer) -> Option<u64> {
			$next(rx).await
		})*
	};
}

async fn level1(rx: &mut Consumer) -> Option<u64> {
	rx.recv().await
}

levels! {
	level2 => level1,
	level3 => level2,
	level4 => level3,
	level5 => level4,
	level6 => level5,
	level7 => level6,
	level8 => level7,
	level9 => level8,
	level10 => level9,
	level11 => level10,
	level12 => level11,
	level13 => level12,
	level14 => level13,
	level15 => level14,
}
