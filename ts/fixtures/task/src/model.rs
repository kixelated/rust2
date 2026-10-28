//! What the harness spawns and drives: a consumer loop holding guards across its awaits,
//! the same loop under a stack of async functions, and the channels they read.

use std::future::Future;
use std::ops::ControlFlow;
use std::pin::{Pin, pin};
use std::task::Poll;

use kio::{Closed, Consumer, Producer, Waiter};

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

	pub fn id(&self) -> u32 {
		self.id
	}

	pub fn log(&self) -> &Log {
		&self.log
	}

	/// Log `event` as this guard's.
	pub(crate) fn note(&self, event: &str) {
		self.log.note(format!("g{} {event}", self.id));
	}

	/// Log `event`, then drop: `self`, taken by value, drops as the call returns.
	fn finish(self, event: &str) -> u32 {
		self.note(event);
		self.id
	}

	/// [`Self::finish`], moved out of its box: freeing the box must not drop it again.
	fn unbox(self: Box<Self>, event: &str) -> u32 {
		(*self).finish(event)
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

/// Every value until the channel closes, summed, read as moq reads kio's channels: the
/// first by `Waiter::poll_future` of a pinned `recv` as a `dyn Future`, under
/// `kio::wait`, then each by a `Task` of the crate's that a `Pending` drives, reached
/// through its `DerefMut` between awaits, which polls with [`at`].
pub async fn watch(mut rx: Consumer, guard: Guard) -> u64 {
	let first = {
		let mut next = pin!(rx.recv());
		let mut next: Pin<&mut dyn Future<Output = Option<u64>>> = next.as_mut();
		kio::wait(|waiter| waiter.poll_future(next.as_mut())).await
	};
	guard.note(&format!("first {first:?}"));
	let mut pending = kio::Pending::new(Watch { rx, index: 1 });
	let mut total = first.unwrap_or(0);
	while let Some(value) = (&mut pending).await {
		guard.note(&format!("watched {value}"));
		total += value;
		pending.index += 1;
	}
	let watch = pending.into_inner();
	watch.rx.closed().await;
	guard.note(&format!("watched to {}", watch.index));
	total
}

/// The value at `index` of a channel, read by [`at`].
struct Watch {
	rx: Consumer,
	index: usize,
}

impl kio::Task for Watch {
	type Output = Option<u64>;

	fn poll(&mut self, waiter: &Waiter) -> Poll<Option<u64>> {
		at(&self.rx, self.index, waiter)
	}
}

/// The value at `index`, or `None` once the channel closed without it, as moq's
/// `poll_group` reads its cache: kio's `Consumer::poll` with a closure pending itself, a
/// `Poll<Poll<_>>`, outside until the value arrives, and inside while it is even and the
/// one after it hasn't: then it waits on the channel, and the close drops it.
fn at(rx: &Consumer, index: usize, waiter: &Waiter) -> Poll<Option<u64>> {
	let res = rx.poll(waiter, |queue| match queue.get(index) {
		None => Poll::Pending,
		Some(value) if value % 2 == 0 && queue.len() == index + 1 => Poll::Ready(Poll::Pending),
		Some(value) => Poll::Ready(Poll::Ready(Some(*value))),
	});
	let res = match res {
		Poll::Ready(Ok(res)) => res,
		Poll::Ready(Err(Closed)) => Poll::Ready(None),
		Poll::Pending => Poll::Pending,
	};
	if res.is_pending() && rx.poll_closed(waiter).is_ready() {
		return Poll::Ready(None);
	}
	res
}

/// One value read through the captures only an async block makes: a borrow, which
/// dropping the block must not drop; a copy, which a later change to the original must
/// not reach; a mutable borrow the block replaces whole, and one of a number it writes
/// across an await, which the caller must see; a move on one path only; and a move the
/// block only borrows, which drops when the block returns or is cancelled. Between them,
/// a value of two types that hold each other, and guards held across awaits that read
/// values first.
pub async fn blocks(mut rx: Consumer, guard: Guard, moves: bool) -> u64 {
	let first = Guard::new(guard.id + 200, guard.log.clone());
	let second = Guard::new(guard.id + 300, guard.log.clone());
	drop(async { first.note("borrowed") });
	turns(&guard.log, guard.id % 5);
	temporaries(&guard.log, guard.id);
	// A guard passed beside an await waits it out, as does one assigned to the slot an
	// await picks: a cancel drops both.
	let waited = consume(Guard::new(guard.id + 900, guard.log.clone()), rx.recv().await);
	let mut slots = [
		Guard::new(guard.id + 910, guard.log.clone()),
		Guard::new(guard.id + 920, guard.log.clone()),
	];
	slots[slot(&mut rx, slots.len()).await] = Guard::new(guard.id + 930, guard.log.clone());
	slots[0].note(&format!("slot after {waited}"));
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
	// A number a block borrows mutably across an await, whose writes the caller sees.
	let mut count = guard.id;
	let counted = async {
		count += 1;
		async {}.await;
		count += 1;
		count
	}
	.await;
	guard.note(&format!("count {counted} {count}"));
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
	// A capture copied into a local on each pass of a loop, which then changes the copy:
	// the next pass copies the capture unchanged.
	let base = guard.id;
	let looped = async move {
		let mut total = 0;
		let mut passes = 0;
		while passes < 2 {
			let mut x = base;
			x += 1;
			async {}.await;
			total += x;
			passes += 1;
		}
		total
	}
	.await;
	chosen.note(&format!("looped {looped}"));
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

/// `v`, or 0, plus the id of `guard`, which it drops.
fn consume(guard: Guard, v: Option<u64>) -> u64 {
	u64::from(guard.finish("consumed")) + v.unwrap_or(0)
}

/// The slot the next value picks, of `eval`. Its captures take names a class or strict
/// code reserves.
async fn slot(constructor: &mut Consumer, eval: usize) -> usize {
	match constructor.recv().await {
		Some(v) => v as usize % eval,
		None => 0,
	}
}

/// Guards no binding holds to the end of the scope, each dropping where Rust drops it: a
/// temporary at the end of its statement, a scrutinee after its `match` or `if let`, a
/// boxed temporary, the part a parameter's pattern skips at the end of the call, the rest
/// a pattern moves out of an array, an empty repeat at once, a closure's captures in its
/// call, and an iterator's after its loop.
#[expect(clippy::redundant_pattern_matching, reason = "the `if let` is under test")]
fn temporaries(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	log.note(format!("temporary {}", guard(1000).id));
	let matched = match Some(guard(1010)) {
		Some(..) => 1,
		None => 0,
	};
	if let None = Some(guard(1020)) {
		log.note(format!("unmatched"));
	}
	log.note(format!("boxed temporary {} {matched}", Box::new(guard(1030)).id));
	log.note(format!("skipped {}", first((guard(1040), guard(1050)))));
	let [head, rest @ ..] = [guard(1060), guard(1070), guard(1080)];
	head.note("head");
	rest[1].note("rest");
	let empty = [guard(1090); 0];
	log.note(format!("empty {}", empty.len()));
	let once = guard(1100);
	let finish = move || once.finish("closure");
	log.note(format!("closed {}", finish()));
	let mut sum = 0;
	for n in (Countdown {
		left: 2,
		guard: guard(1110),
	}) {
		sum += n;
	}
	log.note(format!("counted {sum}"));
}

/// The id of the first guard, which it drops; the second drops as the call returns.
fn first((a, _): (Guard, Guard)) -> u32 {
	a.finish("first")
}

/// Counts down, holding a guard the loop drops after its last turn.
struct Countdown {
	left: u32,
	guard: Guard,
}

impl Iterator for Countdown {
	type Item = u32;

	fn next(&mut self) -> Option<u32> {
		if self.left == 0 {
			return None;
		}
		self.left -= 1;
		self.guard.note("count");
		Some(self.left)
	}
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
/// index and moves nothing; a boxed guard read twice, which a read leaves boxed; a guard
/// a method takes by value, which drops inside it, once from a box; and a boxed array of
/// guards that may be absent, which drops them in order, and a `Result` and a
/// `ControlFlow` that may hold a guard, which rustc's glue drops.
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
	let finished = Guard::new(count + 110, log.clone()).finish("finishing");
	log.note(format!("finished {finished}"));
	let unboxed = Box::new(Guard::new(count + 120, log.clone())).unbox("unboxing");
	log.note(format!("unboxed {unboxed}"));
	let stacked = match count % 2 {
		0 => Some(Box::new([
			Guard::new(count + 130, log.clone()),
			Guard::new(count + 140, log.clone()),
		])),
		_ => None,
	};
	log.note(format!("stacked {}", stacked.is_some()));
	let failed = match count % 2 {
		0 => Err::<u32, _>(Guard::new(count + 150, log.clone())),
		_ => Ok(count),
	};
	log.note(format!("failed {}", failed.is_err()));
	let flow = match count % 3 {
		0 => ControlFlow::Break(Guard::new(count + 160, log.clone())),
		_ => ControlFlow::Continue(count),
	};
	log.note(format!("flow {}", flow.is_break()));
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
