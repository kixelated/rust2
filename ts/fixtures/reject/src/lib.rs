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

pub fn iterators(v: &[u8]) -> bool {
	v.iter().map(|&b| (b > 1).then_some(b)).next().is_some()
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
#[allow(dropping_copy_types, reason = "the closure is only named")]
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

/// Every clone of an `Arc` is one JS object: replacing the payload would reach them all.
pub fn replace_shared(dst: &mut std::sync::Arc<u8>, src: std::sync::Arc<u8>) {
	*dst = src;
}

/// `Ready(Some(Pending))` would be `rt.Pending`, as the outer `Pending` is.
pub fn nested_poll(p: std::task::Poll<Option<std::task::Poll<u8>>>) -> bool {
	matches!(p, std::task::Poll::Pending)
}

/// `None` and `Some(Ready(()))` would both be `undefined`.
pub fn ready_unit(p: Option<std::task::Poll<()>>) -> bool {
	matches!(p, None)
}

/// The payload of the `Some` that fails the test drops after the `if let`.
pub fn unmatched() -> u8 {
	if let None = Some(make()) { 1 } else { 0 }
}

/// A future's `Ready(Pending)` would be `rt.Pending`, as its own pending is.
pub async fn pending_output() -> std::task::Poll<u8> {
	std::task::Poll::Pending
}

/// The temporary drops after the `match`, which nothing marks.
pub fn tested() -> u8 {
	let matched = match Some(make()) {
		Some(_) => 1,
		None => 0,
	};
	matched + 1
}

/// A `Pin` is its pointer in JS: replacing the `Arc` would reach every clone.
pub fn replace_pinned(dst: &mut std::pin::Pin<std::sync::Arc<u8>>, src: std::pin::Pin<std::sync::Arc<u8>>) {
	*dst = src;
}

/// The box drops its `Noisy` at the end of the statement, which nothing marks.
pub fn boxed_field() -> u8 {
	Box::new(make()).0
}

/// A guard is its mutex in JS: replacing it would copy the other mutex's value in.
pub fn replace_guard<'a>(dst: &mut std::sync::MutexGuard<'a, u8>, src: std::sync::MutexGuard<'a, u8>) {
	*dst = src;
}

/// An adaptor without a mapping.
pub fn cycled(v: &[u8]) -> usize {
	v.iter().cycle().take(3).count()
}

/// An iterator whose `fold` std's adaptors would call in place of its `next`.
pub struct Folds;

impl Iterator for Folds {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		None
	}

	fn fold<B, F: FnMut(B, u8) -> B>(self, init: B, _f: F) -> B {
		init
	}
}

pub fn folds() -> usize {
	Folds.map(|b| b + 1).count()
}

/// An iterator with a `next_back`, which an adaptor over it would need.
pub struct Backs;

impl Iterator for Backs {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		None
	}
}

impl DoubleEndedIterator for Backs {
	fn next_back(&mut self) -> Option<u8> {
		None
	}
}

pub fn backs() -> usize {
	Backs.rev().count()
}

/// A zip that `count` reads through `fold`, which stops at the shorter side, and a `for`
/// through `next`, which calls the first side's closure once more.
pub fn zipped(a: &[u8], b: &[u8]) -> usize {
	a.iter().map(|x| x + 1).chain(b.iter().copied()).zip(b).count()
}

/// JS compares objects by identity, not by `PartialOrd`.
#[derive(PartialEq, PartialOrd)]
pub struct Key(pub u8);

pub fn keyed(range: std::ops::Range<Key>) -> bool {
	range.contains(&Key(2))
}

/// An iterator whose `size_hint` std's `collect` calls beside its `next`.
pub struct Hinted;

impl Iterator for Hinted {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		None
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(0))
	}
}

pub fn hinted() -> Vec<u8> {
	Hinted.collect()
}

/// An iterator that `fuse` reads on past its end, since it says it stays ended.
pub struct Fused;

impl Iterator for Fused {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		None
	}
}

impl std::iter::FusedIterator for Fused {}

pub fn fused() -> usize {
	Fused.fuse().count()
}

/// The items `next` leaves drop with the iterator, which an `rt.Iter` never drops.
pub fn drained(v: Vec<Noisy>) -> Option<Noisy> {
	v.into_iter().next()
}

/// `count` drops each item it consumes.
pub fn counted() -> usize {
	(0..2).map(Noisy).count()
}

/// An iterator that drops, after the loop.
pub struct Countdown(pub Noisy);

impl Iterator for Countdown {
	type Item = u8;

	fn next(&mut self) -> Option<u8> {
		None
	}
}

pub fn looped(c: Countdown) -> u8 {
	let mut n = 0;
	for i in c {
		n += i;
	}
	n
}

/// `unwrap_or` drops its fallback when the option holds a value.
pub fn fallback(a: Option<Noisy>, b: Noisy) -> Noisy {
	a.unwrap_or(b)
}

/// `rest` moves the tail out of `pair`, which would still drop it.
pub fn rest_moved(pair: [Noisy; 2]) -> u8 {
	let [first, rest @ ..] = pair;
	first.0 + rest[0].0
}

/// A pinned reference is its referent in JS: replacing it would change what it points at.
pub fn replace_pinned_ref<'a>(dst: &mut std::pin::Pin<&'a mut Noisy>, src: std::pin::Pin<&'a mut Noisy>) {
	*dst = src;
}

/// `[n; 0]` drops `n` at once, which no element of the array would.
pub fn repeated(n: Noisy) -> usize {
	let empty = [n; 0];
	empty.len()
}
