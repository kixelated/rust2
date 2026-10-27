//! Constructs outside the subset: each root fails translation with a span, and no
//! output is written.

#![allow(dead_code, reason = "each item is a translation root, not called from Rust")]

pub fn float(a: f64) -> f64 {
	a * 2.0
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

/// Dropping the closure would drop what it captured, which lives in an arrow function.
pub fn owning_closure() -> u8 {
	let n = Noisy(1);
	let f = move || std::mem::size_of_val(&n) as u8;
	f()
}

/// Each element drops at the end of its iteration, and the rest on a `break`.
pub fn each(v: Vec<Noisy>) -> u8 {
	let mut sum = 0;
	for n in v {
		sum += n.0;
	}
	sum
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

/// A future's `Ready(Pending)` would be `rt.Pending`, as its own pending is.
pub async fn pending_output() -> std::task::Poll<u8> {
	std::task::Poll::Pending
}

/// A `Pin` is its pointer in JS: replacing the `Arc` would reach every clone.
pub fn replace_pinned(dst: &mut std::pin::Pin<std::sync::Arc<u8>>, src: std::pin::Pin<std::sync::Arc<u8>>) {
	*dst = src;
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

/// `rest` borrows the tail of `items` mutably, but a JS array has no views.
pub fn rest_mut(items: &mut [u32; 3]) -> u32 {
	if let [_, rest @ ..] = items {
		rest[0] = 9;
	}
	items[1]
}

/// A pinned reference is its referent in JS: replacing it would change what it points at.
pub fn replace_pinned_ref<'a>(dst: &mut std::pin::Pin<&'a mut Noisy>, src: std::pin::Pin<&'a mut Noisy>) {
	*dst = src;
}

/// A reference copied into a closure holds its referent's object: replacing through it
/// would change the copy, not the referent.
pub fn replace_captured(a: &mut [u8; 1]) -> [u8; 1] {
	let mut g = move || std::mem::replace(&mut *a, [9]);
	g()
}

/// Each copy of a closure has its own captures, which copies of an arrow function share.
pub fn copied_closure(n: u8) -> u8 {
	let mut a = [n];
	let mut g = move || {
		a[0] = a[0].wrapping_add(1);
		a[0]
	};
	let mut h = g;
	g().wrapping_add(h())
}

/// Struct update syntax on a one-sided range reads its base's field, which is not supported.
pub fn one_sided(data: &[u8], to: std::ops::RangeTo<usize>, from: std::ops::RangeFrom<usize>) -> usize {
	data[std::ops::RangeTo { ..to }].len() + data[std::ops::RangeFrom { ..from }].len()
}

/// `get`'s `None` and `Some(&None)` would both be `undefined`.
pub fn optional(map: &std::collections::HashMap<u8, Option<u8>>) -> bool {
	map.get(&1).is_some()
}

/// A key the runtime has no `Hash` and `Eq` of.
pub fn byte_keys() -> usize {
	let map: std::collections::HashMap<Vec<u8>, u8> = std::collections::HashMap::new();
	map.len()
}

/// A key whose `Hash` and `Eq` agree with a `u8`'s, but which is a JS object.
#[derive(PartialEq, Eq, Hash)]
pub struct Id {
	pub n: u8,
}

impl std::borrow::Borrow<u8> for Id {
	fn borrow(&self) -> &u8 {
		&self.n
	}
}

/// Looked up by a `u8`, which the map would look for among its primitive keys.
pub fn borrowed(map: &std::collections::HashMap<Id, u8>) -> bool {
	map.contains_key(&1)
}

#[derive(PartialEq, Eq)]
pub struct Raw(pub u8);

impl std::hash::Hash for Raw {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		state.write(&[self.0]);
	}
}

/// A `Hash` that writes bytes, which the runtime's hasher does not take.
pub fn raw_keys() -> usize {
	let map: std::collections::HashMap<Raw, u8> = std::collections::HashMap::new();
	map.len()
}

pub struct Picky(pub u8);

impl PartialEq for Picky {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}

	fn ne(&self, other: &Self) -> bool {
		self.0 > other.0
	}
}

/// An `Arc`'s `!=` calls `Picky`'s own `ne`, which `!eq` would not.
pub fn picky(a: &std::sync::Arc<Picky>, b: &std::sync::Arc<Picky>) -> bool {
	a != b
}

/// `max` drops the operand it doesn't return, which the runtime's leaves undropped.
pub fn widest(a: std::sync::Arc<str>, b: std::sync::Arc<str>) -> std::sync::Arc<str> {
	a.max(b)
}

/// A reference to a value a map holds that JS copies, passed on, where no `rt.Place` follows it.
pub fn passed(map: &mut std::collections::HashMap<u8, u8>) -> u8 {
	map.get_mut(&1).map_or(0, |v| {
		*v += 1;
		*v
	})
}

/// A `String` pushed to through a parameter, whose JS value is the caller's copy.
pub fn pushed(s: &mut String) {
	s.push('a');
}

/// A `str` pattern, where only a `char` and a predicate are supported.
pub fn split_str(s: &str) -> usize {
	s.split("ab").count()
}
