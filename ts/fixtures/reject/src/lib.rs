//! Constructs outside the subset: each root fails translation with a span, and no
//! output is written.

#![allow(dead_code, reason = "each item is a translation root, not called from Rust")]
#![feature(cow_is_borrowed)]

pub fn float(a: f32) -> f32 {
	a * 2.0
}

pub fn dynamic(f: &mut dyn Iterator<Item = u8>) -> Option<u8> {
	f.next()
}

/// `None` and `Some(None)` would both be `rt.None`, as `Some(Some(None))` is `undefined`.
pub fn nested(x: Option<Option<Option<u8>>>) -> bool {
	x.is_some()
}

pub fn rotated(v: &mut [u8]) {
	v.rotate_left(1);
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

static mut COUNT: u8 = 0;

pub fn statics() -> u8 {
	unsafe { COUNT }
}

pub struct Meters(pub u64);

pub fn generic<T: Copy + Into<u64>>(t: &mut T) -> f32 {
	// A census lowers this over `T`: what hinges on `T` waits for an instance, the float does not.
	(*t).into() as f32
}

/// `Ready(Some(Ready(Pending)))` would be `rt.OuterPending`, as the outer `Pending` is.
pub fn nested_poll(p: std::task::Poll<Option<std::task::Poll<std::task::Poll<u8>>>>) -> bool {
	matches!(p, std::task::Poll::Pending)
}

/// A future's `Ready(Pending)` would be `rt.Pending`, as its own pending is.
pub async fn pending_output() -> std::task::Poll<u8> {
	std::task::Poll::Pending
}

/// A guard is its mutex in JS: replacing it would copy the other mutex's value in.
pub fn replace_guard<'a>(dst: &mut std::sync::MutexGuard<'a, u8>, src: std::sync::MutexGuard<'a, u8>) {
	*dst = src;
}

/// A boxed `PollFn` is the `rt.PollFn`, whose callback is private where a copy can't reach.
pub fn replace_boxed_poll_fn(
	dst: &mut std::pin::Pin<Box<std::future::PollFn<fn(&mut std::task::Context<'_>) -> std::task::Poll<u8>>>>,
	src: std::pin::Pin<Box<std::future::PollFn<fn(&mut std::task::Context<'_>) -> std::task::Poll<u8>>>>,
) -> std::pin::Pin<Box<std::future::PollFn<fn(&mut std::task::Context<'_>) -> std::task::Poll<u8>>>> {
	std::mem::replace(dst, src)
}

/// A boxed byte array is a `Uint8Array`, which a replacement can't copy out as an object.
pub fn replace_boxed_bytes(
	dst: &mut std::pin::Pin<Box<[u8; 2]>>,
	src: std::pin::Pin<Box<[u8; 2]>>,
) -> std::pin::Pin<Box<[u8; 2]>> {
	std::mem::replace(dst, src)
}

/// A boxed future is whatever class its value's type is.
pub fn replace_boxed_future(
	dst: &mut std::pin::Pin<Box<dyn std::future::Future<Output = u8>>>,
	src: std::pin::Pin<Box<dyn std::future::Future<Output = u8>>>,
) {
	*dst = src;
}

/// Swapping two guards would swap their mutexes' values instead.
pub fn swap_guards<'a>(a: &mut std::sync::MutexGuard<'a, u8>, b: &mut std::sync::MutexGuard<'a, u8>) {
	std::mem::swap(a, b);
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

/// A set's `get` of an item that can be `undefined`: its `None` and `Some(&None)` would both be.
pub fn optional(set: &std::collections::HashSet<Option<u8>>) -> bool {
	set.get(&None).is_some()
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

/// An owner that drops, which would drop with the last clone of the `Bytes`.
pub fn owned_bytes(owner: std::sync::Arc<[u8]>) -> usize {
	bytes::Bytes::from_owner(owner).len()
}

/// A `BytesMut`'s capacity, which `bytes` reclaims only once the `Bytes` it froze drops.
pub fn bytes_mut_capacity(n: usize) -> usize {
	let mut buf = bytes::BytesMut::with_capacity(n);
	buf.extend_from_slice(b"frame");
	drop(buf.split_to(2).freeze());
	buf.reserve(n);
	buf.capacity()
}

/// `bytes` reads a `Buf` of the crate's own into a `BytesMut` by its `copy_to_bytes` only
/// at zero capacity.
pub fn bytes_mut_put(buf: &mut bytes::BytesMut, src: Counted) {
	bytes::BufMut::put(buf, src);
}

/// A `Buf` whose `copy_to_bytes` differs from its `chunk` and `advance`.
pub struct Counted(pub bytes::Bytes);

impl bytes::Buf for Counted {
	fn remaining(&self) -> usize {
		self.0.remaining()
	}

	fn chunk(&self) -> &[u8] {
		self.0.chunk()
	}

	fn advance(&mut self, cnt: usize) {
		self.0.advance(cnt);
	}

	fn copy_to_bytes(&mut self, len: usize) -> bytes::Bytes {
		bytes::Bytes::from(vec![0; len])
	}
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

/// Integer parsing is not translated yet.
pub fn int_parsed(s: &str) -> Result<u32, std::num::ParseIntError> {
	s.parse()
}

pub trait Reading {
	fn read(&self) -> u8;
}

impl Reading for u8 {
	fn read(&self) -> u8 {
		*self
	}
}

/// A `Weak` of a trait object, which would need an object made of a value that may be
/// gone.
pub fn weak_object(n: std::sync::Weak<u8>) -> Option<u8> {
	let r: std::sync::Weak<dyn Reading> = n;
	r.upgrade().map(|r| r.read())
}

pub trait Scaled<T> {
	fn scale(&self, by: T) -> T;
}

/// A trait object of a generic trait.
pub fn generic_object(s: &dyn Scaled<u8>) -> u8 {
	s.scale(2)
}

pub trait First {
	fn first(&self) -> u8;
}

impl First for [u8; 4] {
	fn first(&self) -> u8 {
		self[0]
	}
}

/// A trait object of a byte array, which a box holds as a `Uint8Array` and a reference as
/// an `rt.ByteSlice`.
pub fn first_of(n: u8) -> u8 {
	let b: Box<dyn First> = Box::new([n, 8, 9, 10]);
	b.first()
}

pub struct Tail<T: ?Sized> {
	pub value: T,
}

/// A struct whose last field unsizes to a trait object.
pub fn tail_read(t: &Tail<u8>) -> u8 {
	let r: &Tail<dyn Reading> = t;
	r.value.read()
}

/// A generic type's `Poll<T>` is `rt.Poll<T>`, which can't hold `rt.OuterPending`, the
/// `Pending` of a `Poll<Poll<u8>>`.
pub struct Waiting<T> {
	pub poll: std::task::Poll<T>,
}

pub fn generic_pending(poll: std::task::Poll<std::task::Poll<u8>>) -> bool {
	Waiting { poll }.poll.is_pending()
}

/// A copy of a closure that rebinds a shared reference it captured: each Rust copy has
/// its own, where JS would share one binding.
pub fn rebound_reference(a: &u32, b: &u32) -> (u32, u32) {
	let mut r = a;
	let c = move || {
		let old = *r;
		r = b;
		old
	};
	let mut x = c;
	let mut y = c;
	(x(), y())
}

/// `get_mut` of a mutex of a number, whose `&mut` the mapping would copy.
pub fn mutex_number(m: &mut std::sync::Mutex<u8>) -> u8 {
	let n = m.get_mut().unwrap();
	*n += 1;
	*n
}

pub trait Keyed: std::any::Any {
	fn key(&self) -> u8;
}

impl Keyed for u8 {
	fn key(&self) -> u8 {
		*self
	}
}

/// An upcast to `dyn Any`, whose downcast checks for the class of the value's own type,
/// not the object of another trait.
pub fn any_upcast(k: &dyn Keyed) -> bool {
	let any: &dyn std::any::Any = k;
	any.is::<u8>()
}

/// A `TypeId`, which no value stands for yet.
pub fn any_type_id(a: &dyn std::any::Any) -> bool {
	a.type_id() == std::any::TypeId::of::<u8>()
}

pub trait Source {
	type Item: Reading;
	fn item(&self) -> &Self::Item;
}

/// A trait object of a type only an instance decides, which a census lowering this over
/// its type parameter can't make.
pub fn source_reading<S: Source>(s: &S) -> u8 {
	let r: &dyn Reading = s.item();
	r.read()
}

/// A pointer read as another type's bytes.
pub fn reinterpreted(n: u32) -> u8 {
	unsafe { *(&raw const n).cast::<u8>() }
}

/// A pointer's address, as an integer.
pub fn addressed(n: u32) -> usize {
	(&raw const n).addr()
}

/// Memory from the allocator, which has no JS object to point at.
pub fn allocated() -> u8 {
	let layout = std::alloc::Layout::new::<u8>();
	unsafe {
		let p = std::alloc::alloc(layout);
		*p = 1;
		let n = *p;
		std::alloc::dealloc(p, layout);
		n
	}
}

/// A pointer to a `Vec`, whose JS array is its heap, not where Rust keeps the `Vec`.
pub fn vec_pointer(v: &mut Vec<u8>) -> usize {
	let p = &raw const *v;
	unsafe { (*p).len() }
}

/// A union whose fields overlap in values JS keeps apart.
pub union Bits {
	pub int: u32,
	pub float: f64,
}

pub fn bits(n: u32) -> u32 {
	let b = Bits { int: n };
	unsafe { b.int }
}

/// Two `rt.Place`s of one number, which are two objects.
pub fn places_compared(n: &mut u32) -> bool {
	let a = &raw mut *n;
	let b = &raw mut *n;
	a == b
}

pub struct Span2 {
	pub start: (u32, u32),
	pub len: u32,
}

/// A pointer to a part kept as an object, which a write of the whole part elsewhere
/// replaces with another object.
pub fn part_replaced(n: u32) -> u32 {
	let mut s = Span2 { start: (n, n), len: n };
	let p = &raw mut s.start;
	s.start = (0, 0);
	unsafe { (*p).0 + s.len }
}

/// A pointer to a variable's object, which an assignment of the whole variable replaces.
pub fn variable_reassigned(n: u32) -> u32 {
	let mut pair = (n, n);
	let p = &raw mut pair;
	pair = (0, 0);
	unsafe { (*p).0 + pair.1 }
}

/// A pointer to an element kept as an object.
pub fn element_pointer(n: u32) -> u32 {
	let mut pairs = [(n, n); 2];
	let p = &raw mut pairs[1];
	unsafe { (*p).0 }
}

/// Two pointers to values of no bytes, which Rust may put anywhere.
pub fn units_compared() -> bool {
	let unit = || std::ptr::NonNull::<()>::dangling().as_ptr();
	std::ptr::eq(unit(), unit())
}

pub enum Only {
	One,
}

/// A dangling pointer to an enum of no bytes, whose value is its variant's tag.
pub fn dangling_enum() -> bool {
	unsafe { std::ptr::NonNull::<Only>::dangling().as_ptr().as_ref().is_some() }
}

/// A buffer of the crate's own that JS copies, so `&mut` to it is an `rt.Place`, which the
/// provided `copy_to_bytes` doesn't read through.
pub enum Blank {
	Empty,
}

impl bytes::Buf for Blank {
	fn remaining(&self) -> usize {
		0
	}

	fn chunk(&self) -> &[u8] {
		&[]
	}

	fn advance(&mut self, cnt: usize) {
		assert_eq!(cnt, 0);
	}
}

pub fn blank_bytes(len: usize) -> usize {
	bytes::Buf::copy_to_bytes(&mut Blank::Empty, len).len()
}

/// A heap's `pop` of an item that can be `undefined`: its `None` and `Some(None)` would both be.
pub fn heap_optional(heap: &mut std::collections::BinaryHeap<Option<u8>>) -> bool {
	heap.pop().is_some()
}

/// The places `iter_mut` lends, which the runtime would compare as the objects they are.
pub fn places_sorted(v: &mut [u8]) -> bool {
	v.iter_mut().is_sorted()
}

/// A pointer into memory the translation can't name.
static NAMED: &u32 = &5;

static SLOT: std::sync::atomic::AtomicPtr<u32> = std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

pub fn pointer_statics(p: *mut u32) -> u32 {
	use std::sync::atomic::Ordering::Relaxed;
	// Two `rt.Place`s of one place are two objects, which `===` can't tell are one.
	let _ = SLOT.compare_exchange(p, std::ptr::null_mut(), Relaxed, Relaxed);
	*NAMED
}

/// Ranges of `Bound`s, which the runtime reads as unbounded where it reads no `start` or `end`.
pub fn bound_ranges(v: &mut Vec<u8>, map: &std::collections::BTreeMap<u8, u8>) -> usize {
	use std::ops::Bound::{Excluded, Included};
	v.drain((Excluded(0), Included(2))).count() + map.range((Excluded(0), Included(2))).count()
}

/// A census lowers this over `F` and `O`, and `awaited`'s body with it, whose closure names
/// `O` as this function's second type parameter: that refuses nothing.
pub async fn generic_awaited<F: Fn() -> u8, O: std::future::Future<Output = u8>>(f: F, o: O) -> u8 {
	f() + awaited(o).await
}

async fn awaited<O: std::future::Future<Output = u8>>(o: O) -> u8 {
	let mut o = std::pin::pin!(o);
	std::future::poll_fn(|cx| poll_pinned(o.as_mut(), cx)).await
}

fn poll_pinned<O: std::future::Future>(
	o: std::pin::Pin<&mut O>,
	cx: &mut std::task::Context<'_>,
) -> std::task::Poll<O::Output> {
	o.poll(cx)
}
