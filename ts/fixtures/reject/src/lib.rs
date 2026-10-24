//! Constructs outside the subset: each root fails translation with a span, and no
//! output is written.

#![allow(dead_code, reason = "each item is a translation root, not called from Rust")]

pub fn float(a: f64) -> f64 {
	a * 2.0
}

pub fn character(c: char) -> u32 {
	c as u32
}

pub fn dynamic(f: &dyn Fn(u8) -> u8) -> u8 {
	f(1)
}

/// JS has no reference to a number: the caller's `u32` would not change.
pub fn bump(x: &mut u32) {
	*x += 1;
}

/// Each element would be a copy, so the increments would be lost.
pub fn increment(v: &mut [u32]) {
	for x in v {
		*x += 1;
	}
}

/// `None` and `Some(None)` would both be `undefined`.
pub fn nested(x: Option<Option<u8>>) -> bool {
	x.is_some()
}

pub fn sorted(v: &mut [u8]) {
	v.sort();
}

pub fn iterators(v: &[u8]) -> usize {
	v.iter().filter(|b| **b > 1).count()
}

pub struct Noisy(pub u8);

impl Drop for Noisy {
	fn drop(&mut self) {}
}

impl Noisy {
	/// A move out of `self` would leave `this` undefined, which JS can't.
	pub fn into_inner(self) -> u8 {
		self.0
	}
}

pub fn make() -> Noisy {
	Noisy(1)
}

/// The temporary drops at the end of the statement, which nothing in JS marks.
pub fn temporary() -> u8 {
	make().0
}

/// Dropping the closure would drop what it captured.
pub fn owning_closure() -> u8 {
	let n = make();
	let f = move || drop(n);
	f();
	0
}

/// Each element drops at the end of its iteration, and the rest on a `break`.
pub fn each(v: Vec<Noisy>) -> u8 {
	let mut sum = 0;
	for n in v {
		sum += n.0;
	}
	sum
}

/// The part the pattern skips drops at the end of the function, which nothing marks.
pub fn destructured((a, _): (Noisy, Noisy)) -> u8 {
	a.0
}

pub enum Slot {
	Empty,
	Full(Noisy),
}

/// An enum's drop glue would switch on its variant.
pub fn slot(s: Slot) -> bool {
	matches!(s, Slot::Empty)
}

pub struct Boxed<T>(pub T);

impl<T> Drop for Boxed<T> {
	fn drop(&mut self) {}
}

/// A generic `Drop` impl has an instance per type.
pub fn boxed() -> u8 {
	let b = Boxed(1u8);
	b.0
}

/// The async block's generator would change its own copy of `n`.
pub fn async_borrow() -> u8 {
	let mut n = 1u8;
	let f = async {
		n += 1;
	};
	drop(f);
	n
}

/// An async closure is a coroutine closure, not an async body.
pub fn async_closure() -> u8 {
	let f = async || 1u8;
	drop(f);
	0
}

/// Format specs outside what moq uses.
pub fn specs(n: u8, w: usize) -> [String; 9] {
	[
		format!("{n:.2}"),
		format!("{n:+}"),
		format!("{n:-}"),
		format!("{n:b}"),
		format!("{n:o}"),
		format!("{n:e}"),
		format!("{n:x?}"),
		format!("{n:w$}"),
		format!("{:p}", &n),
	]
}

/// `fmt::Write` for a `String`, which a JS string can't be changed through.
pub fn written(s: &mut String, n: u8) {
	use std::fmt::Write;
	let _ = write!(s, "{n}");
}

pub fn captured() -> u8 {
	let mut n = 1;
	let f = move || n + 1;
	n += 10;
	f() + n
}

pub fn or_bindings(x: Result<u8, u8>) -> u8 {
	match x {
		Ok(v) | Err(v) => v,
	}
}

static COUNT: u8 = 0;

pub fn statics() -> u8 {
	COUNT
}

/// The path to `*a` is `a`: assigning it would rebind the parameter, not change the caller's.
pub fn swapped(a: &mut Meters, b: &mut Meters) {
	std::mem::swap(a, b);
}

pub fn replaced(a: &mut Meters) -> Meters {
	std::mem::replace(&mut *a, Meters(0))
}

pub struct Meters(pub u64);

pub fn generic<T: Copy + Into<u64>>(t: &mut T) -> f64 {
	// A census lowers this over `T`: what hinges on `T` waits for an instance, the float does not.
	(*t).into() as f64
}

/// An `Arc` of a borrow is the borrowed value in JS: dropping the last one would drop it.
pub fn shared_borrow(n: &Noisy) -> usize {
	let a = std::sync::Arc::new(n);
	std::sync::Arc::strong_count(&a)
}

/// `n` would wait out the `.await` where no scope drops it if the future is cancelled.
pub async fn held_across(n: Noisy) -> u8 {
	consume(n, async { 1u8 }.await)
}

fn consume(n: Noisy, v: u8) -> u8 {
	drop(n);
	v
}

/// The skipped element drops at the end of the statement, which nothing marks.
pub fn skipped() -> u8 {
	let [kept, _] = [make(), make()];
	kept.0
}

/// `n` would wait out the `.await` in the index where no scope drops it on a cancel.
pub async fn assigned(n: Noisy) -> u8 {
	let mut slots = [make(), make()];
	slots[index().await] = n;
	slots[0].0
}

async fn index() -> usize {
	1
}

/// `Some(..)` leaves the payload in the temporary, which drops after the `match`.
pub fn unnamed() -> u8 {
	match Some(make()) {
		Some(..) => 1,
		None => 0,
	}
}
