//! Constructs outside the subset: each root fails translation with a span, and no
//! output is written.

#![allow(dead_code, reason = "each item is a translation root, not called from Rust")]
#![feature(cow_is_borrowed)]

pub fn float(a: f32) -> f32 {
	a * 2.0
}

pub fn dynamic(f: &dyn Fn(u8) -> u8) -> u8 {
	f(1)
}

/// Each element would be a copy, so the increments would be lost.
pub fn increment(v: &mut [u32]) {
	for x in v {
		*x += 1;
	}
}

/// `None` and `Some(None)` would both be `rt.None`, as `Some(Some(None))` is `undefined`.
pub fn nested(x: Option<Option<Option<u8>>>) -> bool {
	x.is_some()
}

pub fn sorted(v: &mut [u8]) {
	v.sort();
}

/// A generic type's `Option<T>` is `T | undefined`, which can't hold `rt.None`, the `None`
/// of an `Option<()>`.
pub struct Maybe<T> {
	pub value: Option<T>,
}

pub fn generic_unit(value: Option<()>) -> bool {
	Maybe { value }.value.is_some()
}

/// The runtime's collecting into an `Option` ends at the first `undefined`, which `Some(())` is.
pub fn collected(v: &[u8]) -> Option<Vec<()>> {
	v.iter().map(|&b| (b > 1).then_some(())).collect()
}

/// The runtime's `find` takes `None` to be `undefined`, which `Some(None)` is.
pub fn iterators(v: &[u8]) -> bool {
	v.iter().map(|&b| (b > 1).then_some(b)).find(Option::is_none).is_some()
}

pub struct Noisy(pub u8);

impl Drop for Noisy {
	fn drop(&mut self) {}
}

/// std's `fold` of `into_keys` drops each value after its key, where `next` drops it first.
pub fn keys(m: std::collections::HashMap<u8, Noisy>) -> u8 {
	let mut sum = 0;
	for k in m.into_keys() {
		sum += k;
	}
	sum
}

/// An async closure is a coroutine closure, not an async body.
pub fn async_closure() -> u8 {
	let f = async || 1u8;
	drop(f);
	0
}

/// A pointer's address, which JS has no counterpart of.
pub fn pointer(n: u8) -> String {
	format!("{:p}", &n)
}

/// `fmt::Write` for a `String`, which has no mapping.
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

pub fn generic<T: Copy + Into<u64>>(t: &mut T) -> f32 {
	// A census lowers this over `T`: what hinges on `T` waits for an instance, the float does not.
	(*t).into() as f32
}

/// Every clone of an `Arc` is one JS object: replacing the payload would reach them all.
pub fn replace_shared(dst: &mut std::sync::Arc<u8>, src: std::sync::Arc<u8>) {
	*dst = src;
}

/// `Ready(Some(Pending))` would be `rt.Pending`, as the outer `Pending` is.
pub fn nested_poll(p: std::task::Poll<Option<std::task::Poll<u8>>>) -> bool {
	matches!(p, std::task::Poll::Pending)
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

/// std's `zip` of two `TrustedRandomAccess` iterators makes one item past the shorter, and drops it.
pub fn zip_dropped(v: &[u8]) -> usize {
	v.iter().map(|&n| Noisy(n)).zip(0..2).count()
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
pub fn replace_captured(a: &mut [u16; 1]) -> [u16; 1] {
	let mut g = move || std::mem::replace(&mut *a, [9]);
	g()
}

/// A std adaptor lends its callback a reference to each array of bytes it holds, which it
/// would pass as the array, not a slice of it.
pub fn filter_owned_bytes(v: Vec<[u8; 2]>) -> usize {
	v.into_iter().filter(|p| p[0] != 0).count()
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

/// std hands its closure a `&mut` to a value JS copies, which its mapping has as the value.
pub fn passed(map: &mut std::collections::HashMap<u8, u8>) -> u8 {
	map.get_mut(&1).map_or(0, |v| {
		*v += 1;
		*v
	})
}

/// JS's `copyWithin` would share each tuple, which Rust copies.
pub fn shifted(v: &mut [(u8, u8)]) {
	v.copy_within(0..1, 1);
}

/// An `Arc`'s bytes would need a `ByteSlice` made inside it.
pub fn shared(b: std::sync::Arc<[u8; 2]>) -> std::sync::Arc<[u8]> {
	b
}

/// A `hash_slice` of its own, which a `Vec`'s `Hash` calls where the translation hashes each
/// element.
#[derive(PartialEq, Eq)]
pub struct Batched(u8);

impl std::hash::Hash for Batched {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.0.hash(state);
	}

	fn hash_slice<H: std::hash::Hasher>(data: &[Self], state: &mut H) {
		state.write_usize(data.len());
	}
}

pub fn batched(v: Vec<Batched>) -> usize {
	let mut set = std::collections::HashSet::new();
	set.insert(v);
	set.len()
}

/// Closures that own a value that drops, which `or_insert_with` would call but never drop
/// if the key is there, and which `for_each` drops before the items it doesn't reach.
pub fn called_closure(v: Vec<Noisy>, m: &mut std::collections::HashMap<u8, Noisy>) {
	let (a, b) = (Noisy(1), Noisy(2));
	m.entry(1).or_insert_with(move || b);
	v.into_iter().for_each(move |x| assert!(x.0 != (&a).0));
}

/// A closure that owns a value that drops, given as a `Pattern`, which `contains` would
/// call but never drop.
pub fn pattern_closure(s: &str) -> bool {
	let n = Noisy(1);
	s.contains(move |c: char| c as u8 == (&n).0)
}

/// A `Cow<str>` is a JS string, which keeps no variant to match on or ask about.
pub fn cow_matched(c: std::borrow::Cow<'_, str>) -> String {
	match c {
		std::borrow::Cow::Borrowed(s) => s.to_string(),
		std::borrow::Cow::Owned(s) => s,
	}
}

pub fn cow_borrowed(c: &std::borrow::Cow<'_, str>) -> bool {
	std::borrow::Cow::is_borrowed(c)
}

/// Only a `Cow<str>` has a JS value yet.
pub fn cow_bytes(c: std::borrow::Cow<'_, [u8]>) -> usize {
	c.len()
}

/// Made where its type goes unchecked, as a local's, it is refused where it is made.
pub fn cow_made(a: &std::sync::Arc<u8>) -> usize {
	{
		let c: std::borrow::Cow<'_, std::sync::Arc<u8>> = std::borrow::Cow::Owned(a.clone());
		drop(c);
	}
	std::sync::Arc::strong_count(a)
}

/// std makes a `&mut` to a value JS copies, which its mapping has as the value.
pub fn last_bumped(v: &mut Vec<u32>) {
	if let Some(x) = v.last_mut() {
		*x += 1;
	}
}

/// std hands the closure a `&mut` to a value JS copies, as `passed`'s is.
pub fn modified(map: &mut std::collections::HashMap<u8, u32>) {
	map.entry(1).and_modify(|v| *v += 1);
}

fn bumped(x: &mut u8) {
	*x += 1;
}

/// A `Result`'s value passed on: a store through it makes the `Result` again, which no
/// `rt.Place` of a path does.
pub fn ok_passed(r: &mut Result<u8, u8>) {
	if let Ok(v) = r {
		bumped(v);
	}
}

/// `Option`s holding `rt.Place`s, which std's runtime would compare as the Places.
pub fn places_equal(a: &mut u32, b: &mut u32) -> bool {
	Some(a) == Some(b)
}

pub struct Part(pub u64);

impl std::iter::Sum<Part> for std::time::Duration {
	fn sum<I: Iterator<Item = Part>>(parts: I) -> Self {
		std::time::Duration::from_secs(parts.map(|p| p.0 * 2).sum())
	}
}

impl std::iter::Sum<Part> for u64 {
	fn sum<I: Iterator<Item = Part>>(parts: I) -> Self {
		parts.map(|p| p.0 * 2).sum()
	}
}

/// A `Sum` of the crate's into a std type, which std's `Sum` mapping would bypass.
pub fn parts_summed(a: Vec<Part>, b: Vec<Part>) -> (std::time::Duration, u64) {
	(a.into_iter().sum(), b.into_iter().sum())
}

/// One rounding, which JS has no cheap form of.
pub fn fma(a: f64) -> f64 {
	a.mul_add(2.0, 1.0)
}

/// Float formatting is not translated yet.
pub fn float_printed(a: f64) -> String {
	format!("{a}")
}

/// An `Arc` around a future of the crate's: a `dyn Future` behind it would need the `Arc`'s
/// own count around an `rt.Future`.
pub fn shared_future() -> std::sync::Arc<dyn std::future::Future<Output = u8>> {
	std::sync::Arc::new(Ready(1))
}

pub struct Ready(u8);

impl std::future::Future for Ready {
	type Output = u8;

	fn poll(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<u8> {
		std::task::Poll::Ready(self.0)
	}
}
