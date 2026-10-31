//! Constructs outside the subset: each root fails translation with a span, and no
//! output is written.

#![allow(dead_code, reason = "each item is a translation root, not called from Rust")]
#![feature(cow_is_borrowed, f16)]

pub fn float(a: f16) -> f16 {
	a * 2.0
}

pub fn dynamic(f: &mut dyn Iterator<Item = u8>) -> Option<u8> {
	f.next()
}

/// `None` and `Some(None)` would both be `rt.None`, as `Some(Some(None))` is `undefined`.
pub fn nested(x: Option<Option<Option<u8>>>) -> bool {
	x.is_some()
}

pub fn refilled(v: &mut [Reused], r: Reused) {
	v.fill(r);
}

/// A generic type's `Option<T>` is `T | undefined`, which can't hold `rt.None`, the `None`
/// of an `Option<()>`.
pub struct Maybe<T> {
	pub value: Option<T>,
}

pub fn generic_unit(value: Option<()>) -> bool {
	Maybe { value }.value.is_some()
}

/// `scan` of items that can be `undefined`: its closure's `None` and `Some(None)` would both be.
pub fn scanned(v: &[u8]) -> usize {
	v.iter()
		.scan(Vec::<u8>::new(), |_, &b| Some((b > 1).then_some(b)))
		.count()
}

pub struct Noisy(pub u8);

impl Drop for Noisy {
	fn drop(&mut self) {}
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

/// A placeholder written to the crate's own `fmt::Write`, which sees each call std makes.
pub fn written(w: &mut Sink, n: u8) {
	use std::fmt::Write;
	let _ = write!(w, "{n}");
}

static mut COUNT: u8 = 0;

pub fn statics() -> u8 {
	unsafe { COUNT }
}

pub struct Meters(pub u64);

pub fn generic<T: Copy + Into<u64>>(t: &mut T) -> f16 {
	// A census lowers this over `T`: what hinges on `T` waits for an instance, the float does not.
	(*t).into() as f16
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

/// A `skip` of a zip std reads by index, which a zip by index drops a pair at a time.
pub fn skipped_zip(v: &[u8]) -> usize {
	let pairs = v.iter().map(|&n| Noisy(n)).zip(v.iter().map(|&n| Noisy(!n)));
	pairs.skip(1).zip(0..2).count()
}

/// A zip collected in place, which std reads by index and drops with what its closure owns.
pub fn zip_in_place(v: Vec<u8>) -> Vec<u8> {
	let n = Noisy(1);
	let owned = v.into_iter().map(move |x| x ^ (&n).0);
	owned.zip(0..2u8).map(|(x, y)| x ^ y).collect()
}

/// A pinned reference is its referent in JS: replacing it would change what it points at.
pub fn replace_pinned_ref<'a>(dst: &mut std::pin::Pin<&'a mut Noisy>, src: std::pin::Pin<&'a mut Noisy>) {
	*dst = src;
}

/// `copied` is given a reference to an array of bytes, a slice of it, which its mapping
/// would copy as the array.
pub fn copied_bytes(v: &[[u8; 2]]) -> [u8; 2] {
	v.first().copied().unwrap_or_default()
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

/// `scan` of `()`s: its closure's `None` and `Some(())` would both be `undefined`.
pub fn scanned_unit(v: &[u8]) -> usize {
	v.iter().scan(Vec::<u8>::new(), |_, _| Some(())).count()
}

/// A key the runtime has no `Hash` and `Eq` of.
pub fn set_keys() -> usize {
	let map: std::collections::HashMap<std::collections::BTreeSet<u8>, u8> = std::collections::HashMap::new();
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

/// `clone_from_slice` calls `Reused`'s own `clone_from` for each element.
pub fn recloned(v: &mut [Reused], w: &[Reused]) {
	v.clone_from_slice(w);
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

/// A closure that owns a value that drops, which `or_insert_with` would call but never drop
/// if the key is there.
pub fn called_closure(m: &mut std::collections::HashMap<u8, Noisy>) {
	let b = Noisy(2);
	m.entry(1).or_insert_with(move || b);
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

/// A slice's tells its variants apart, but asking is unstable.
pub fn cow_bytes(c: &std::borrow::Cow<'_, [u8]>) -> bool {
	std::borrow::Cow::is_borrowed(c)
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

/// Parsing an address is not translated yet.
pub fn addr_parsed(s: &str) -> Result<std::net::Ipv4Addr, std::net::AddrParseError> {
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

/// A pointer to a variable's object, which `mem::replace` of the whole variable replaces.
pub fn variable_replaced(n: u32) -> u32 {
	let mut pair = (n, n);
	let p = &raw mut pair;
	let old = std::mem::replace(&mut pair, (0, 0));
	unsafe { (*p).0 + old.1 }
}

/// A pointer to a variable's object, which `mem::swap` replaces.
pub fn variable_swapped(n: u32) -> u32 {
	let mut a = (n, n);
	let mut b = (0, 0);
	let p = &raw mut a;
	std::mem::swap(&mut a, &mut b);
	unsafe { (*p).0 + b.1 }
}

/// A pointer to a variable's object, which a whole write through a `&mut` to it replaces.
pub fn assigned_through(n: u32) -> u32 {
	let mut pair = (n, n);
	let p = &raw mut pair;
	let r = &mut pair;
	*r = (0, 0);
	unsafe { (*p).0 }
}

/// A pointer `NonNull::from` makes of a `&mut` to a variable, which an assignment replaces.
pub fn nonnull_reassigned(n: u32) -> u32 {
	let mut pair = (n, n);
	let p = std::ptr::NonNull::from(&mut pair);
	pair = (0, 0);
	unsafe { p.as_ref().0 + pair.1 }
}

/// A pointer cast from a `&mut` to a variable, which an assignment replaces.
pub fn cast_reassigned(n: u32) -> u32 {
	let mut pair = (n, n);
	let p = &mut pair as *mut (u32, u32);
	pair = (0, 0);
	unsafe { (*p).0 + pair.1 }
}

pub struct Window {
	pub range: (u32, u32),
}

/// A pointer to a part kept as an object, which `mem::replace` of the part elsewhere replaces.
pub fn range_of(w: &mut Window) -> *mut (u32, u32) {
	&raw mut w.range
}

pub fn range_replaced(w: &mut Window) -> (u32, u32) {
	std::mem::replace(&mut w.range, (0, 0))
}

/// A pointer to an element kept as an object.
pub fn element_pointer(n: u32) -> u32 {
	let mut pairs = [(n, n); 2];
	let p = &raw mut pairs[1];
	unsafe { (*p).0 }
}

/// The pointer `UnsafeCell::get` gives to the object its `rt.Cell` holds, which a whole
/// write of the cell elsewhere replaces with another object.
pub fn cell_replaced(n: u32) -> u32 {
	let mut cell = std::cell::UnsafeCell::new((n, n));
	let p = cell.get();
	let old = std::mem::replace(&mut cell, std::cell::UnsafeCell::new((0, 0)));
	unsafe { (*p).0 + old.into_inner().1 }
}

/// The pointer `UnsafeCell::get` gives to a number, the `rt.Cell` itself, which an
/// assignment of the whole variable replaces.
pub fn cell_reassigned(n: u32) -> u32 {
	let mut cell = std::cell::UnsafeCell::new(n);
	let p = cell.get();
	cell = std::cell::UnsafeCell::new(0);
	unsafe { *p + cell.into_inner() }
}

pub struct Hits {
	pub count: std::cell::UnsafeCell<u32>,
}

/// The pointer `UnsafeCell::get` gives to a number, the `rt.Cell` itself, which
/// `mem::take` of the whole variable replaces.
pub fn cell_taken(n: u32) -> u32 {
	let mut cell = std::cell::UnsafeCell::new(n);
	let p = cell.get();
	let old = std::mem::take(&mut cell);
	unsafe { *p + old.into_inner() }
}

/// The pointer `UnsafeCell::get` gives to a number, the `rt.Cell` in a struct, which a
/// write of the whole field replaces.
pub fn cell_field_replaced(n: u32) -> u32 {
	let mut hits = Hits {
		count: std::cell::UnsafeCell::new(n),
	};
	let p = hits.count.get();
	hits.count = std::cell::UnsafeCell::new(0);
	unsafe { *p + *hits.count.get_mut() }
}

/// Pointers to places of no bytes, which Rust may put anywhere, `()`'s refused as made.
pub fn units_compared() -> bool {
	let (a, b, c, d) = ((), (), Fused, Fused);
	std::ptr::eq(&raw const a, std::ptr::NonNull::from(&b).as_ptr()) && std::ptr::eq(&raw const c, &raw const d)
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

/// A closure that owns a value that drops, which `String::retain` would call but never drop.
pub fn string_retained(s: &mut String) {
	let n = Noisy(1);
	s.retain(move |c| c as u8 != (&n).0);
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

pub struct Sink(u8);

impl std::fmt::Write for Sink {
	fn write_str(&mut self, s: &str) -> std::fmt::Result {
		self.0 = self.0.wrapping_add(s.len() as u8);
		Ok(())
	}
}

pub struct Plain {
	pub at: (u32, u32),
}

fn put<T>(slot: &mut T, value: T) {
	*slot = value;
}

/// A pointer to a part kept as an object, which a generic body reached at its struct
/// replaces with another object.
pub fn put_replaced(n: u32) -> u32 {
	let mut plain = Plain { at: (n, n) };
	let p = &raw mut plain.at;
	put(&mut plain, Plain { at: (0, 0) });
	unsafe { (*p).0 }
}

#[derive(Default)]
pub struct Track {
	pub at: (u32, u32),
}

pub struct Bin<T>(std::cell::UnsafeCell<T>);

pub trait Reset {
	fn reset(&self);
}

impl<T: Default> Reset for Bin<T> {
	fn reset(&self) {
		unsafe { *self.0.get() = T::default() };
	}
}

/// A pointer to a part kept as an object, which a generic body reached only through a trait
/// object replaces with another object.
pub fn object_replaced(n: u32) -> u32 {
	let bin = Bin(std::cell::UnsafeCell::new(Track { at: (n, n) }));
	let p = unsafe { &raw mut (*bin.0.get()).at };
	let object: &dyn Reset = &bin;
	object.reset();
	unsafe { (*p).0 }
}

#[derive(Default)]
pub struct Lane {
	pub at: (u32, u32),
}

fn resetter<T: Default>(slot: *mut T) -> impl FnOnce() {
	move || unsafe { *slot = T::default() }
}

/// A pointer to a part kept as an object, which a generic closure reached at its struct
/// replaces with another object.
pub fn closure_replaced(n: u32) -> u32 {
	let mut lane = Lane { at: (n, n) };
	let whole = &raw mut lane;
	let p = unsafe { &raw mut (*whole).at };
	resetter(whole)();
	unsafe { (*p).0 }
}

/// A `clone_from` of its own, which `fill` calls for each slot but the last.
pub struct Reused(Vec<u8>);

impl Clone for Reused {
	fn clone(&self) -> Self {
		Reused(self.0.clone())
	}

	fn clone_from(&mut self, source: &Self) {
		self.0.clone_from(&source.0);
	}
}

pub struct Gap {
	pub range: (u32, u32),
	pub slot: std::mem::MaybeUninit<(u32, u32)>,
}

fn reset_gap(gap: &mut Gap) {
	gap.range = (0, 0);
	gap.slot = std::mem::MaybeUninit::new((0, 0));
}

/// The pointers `NonNull::from` makes of a `&mut` and of a `&` to a part kept as an object,
/// and `into` of a `&mut`, which a write of the whole part elsewhere replaces with another
/// object.
pub fn made_part_replaced(n: u32) -> u32 {
	let mut gap = Gap {
		range: (n, n),
		slot: std::mem::MaybeUninit::new((n, n)),
	};
	let p = std::ptr::NonNull::from(&mut gap.range);
	let q = std::ptr::NonNull::from(&gap.range);
	let r: std::ptr::NonNull<(u32, u32)> = (&mut gap.range).into();
	reset_gap(&mut gap);
	unsafe { p.as_ref().0 + q.as_ref().1 + r.as_ref().0 }
}

/// The pointers `MaybeUninit::as_ptr` and `as_mut_ptr` make to a part kept as an object,
/// which a write of the whole part elsewhere replaces with another object.
pub fn uninit_part_replaced(n: u32) -> u32 {
	let mut gap = Gap {
		range: (n, n),
		slot: std::mem::MaybeUninit::new((n, n)),
	};
	let p = gap.slot.as_ptr();
	let q = gap.slot.as_mut_ptr();
	reset_gap(&mut gap);
	unsafe { (*p).0 + (*q).1 }
}

/// A key ordered by its own `Ord`.
#[derive(Clone, Copy)]
pub struct Ranked(pub u8);

impl PartialEq for Ranked {
	fn eq(&self, other: &Self) -> bool {
		self.0 % 4 == other.0 % 4
	}
}

impl Eq for Ranked {}

impl PartialOrd for Ranked {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Ranked {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(self.0 % 4).cmp(&(other.0 % 4))
	}
}

/// Entries that drop, sorted by what a cell holds, whose own `Ord` the runtime's sort
/// calls where std's does not.
pub fn ranked_cells(entries: Vec<(std::cell::Cell<Ranked>, Noisy)>) -> usize {
	entries.into_iter().collect::<std::collections::BTreeMap<_, _>>().len()
}

impl PartialEq<Key> for u8 {
	fn eq(&self, other: &Key) -> bool {
		*self == other.0
	}
}

impl PartialEq<u8> for Key {
	fn eq(&self, other: &u8) -> bool {
		self.0 == *other
	}
}

impl PartialOrd<Key> for u8 {
	fn partial_cmp(&self, other: &Key) -> Option<std::cmp::Ordering> {
		self.partial_cmp(&other.0)
	}
}

impl PartialOrd<u8> for Key {
	fn partial_cmp(&self, other: &u8) -> Option<std::cmp::Ordering> {
		self.0.partial_cmp(other)
	}
}

/// Integer bounds against the crate's item, which compares by its own `PartialOrd`.
pub fn keyed_across(n: u8) -> bool {
	(..10u8).contains(&Key(n)) || std::ops::RangeBounds::contains(&(1u8..), &Key(n))
}

/// Bounds behind `&mut`s, which JS may keep as places rather than the integers.
pub fn place_bounds(a: &mut u8, b: &mut u8) -> bool {
	(a..b).is_empty()
}

/// A struct holding a pointer.
pub struct Aimed {
	pub at: *mut (u32, u32),
}

/// A pointer to a variable's object, copied into a struct read after an assignment of
/// the whole variable replaces the object.
pub fn pointer_in_struct(n: u32) -> u32 {
	let mut pair = (n, n);
	let aimed = Aimed { at: &raw mut pair };
	pair = (0, 0);
	unsafe { (*aimed.at).0 + pair.1 }
}

fn handed_back<T>(p: *mut T) -> *mut T {
	p
}

/// A pointer to a variable's object, which a call returns, read after an assignment of
/// the whole variable replaces the object.
pub fn pointer_returned(n: u32) -> u32 {
	let mut pair = (n, n);
	let p = handed_back(&raw mut pair);
	pair = (0, 0);
	unsafe { (*p).0 + pair.1 }
}

/// A pointer to a variable's object, kept in a `Vec` and read after an assignment of the
/// whole variable replaces the object.
pub fn pointer_pushed(n: u32) -> u32 {
	let mut pair = (n, n);
	let mut kept = Vec::new();
	kept.push(&raw mut pair);
	pair = (0, 0);
	unsafe { (*kept[0]).0 + pair.1 }
}

/// A pointer to a variable's object, which a closure captures and reads after an
/// assignment of the whole variable replaces the object.
pub fn pointer_captured(n: u32) -> u32 {
	let mut pair = (n, n);
	let p = &raw mut pair;
	let read = move || unsafe { (*p).0 };
	pair = (0, 0);
	read() + pair.1
}

/// A pointer an earlier iteration made to a variable's object, read after the next
/// iteration's assignment of the whole variable replaces the object.
pub fn pointer_kept_across(data: &[u32]) -> u32 {
	let mut pair = (0, 0);
	let mut last = &raw mut pair;
	let mut sum = 0;
	for &n in data {
		pair = (n, n);
		sum += unsafe { (*last).0 };
		last = &raw mut pair;
	}
	sum
}

/// Structs JS keeps as objects, each holding an `rt.Cell` of an object, one for each write
/// of fresh memory made while a pointer into it may still be read, which gives the cell
/// another object.
pub struct Escaped(pub std::cell::UnsafeCell<(u16, u16)>);

pub struct Called(pub std::cell::UnsafeCell<(u16, u16)>);

pub struct Looped(pub std::cell::UnsafeCell<(u16, u16)>);

pub struct Aliased(pub std::cell::UnsafeCell<(u16, u16)>);

pub struct Rewritten(pub std::cell::UnsafeCell<(u16, u16)>);

pub struct Keeper {
	pub whole: *mut Escaped,
	pub part: *mut (u16, u16),
}

fn keep_part(keeper: &mut Keeper) {
	keeper.part = unsafe { (*keeper.whole).0.get() };
}

/// A fresh allocation written whole again after its pointer escaped into a struct.
pub fn escaped_into_struct(n: u16, keeper: &mut Keeper) -> u16 {
	let p = Box::into_raw(Box::new(std::mem::MaybeUninit::<Escaped>::uninit())).cast::<Escaped>();
	unsafe { p.write(Escaped(std::cell::UnsafeCell::new((n, n)))) };
	keeper.whole = p;
	keep_part(keeper);
	unsafe { p.write(Escaped(std::cell::UnsafeCell::new((0, 0)))) };
	unsafe { (*keeper.part).0 }
}

fn kept_part(p: *mut Called, kept: &mut Vec<*mut (u16, u16)>) {
	kept.push(unsafe { (*p).0.get() });
}

/// A fresh allocation written whole again after its pointer escaped into a call.
pub fn escaped_into_call(n: u16) -> u16 {
	let p = Box::into_raw(Box::new(std::mem::MaybeUninit::<Called>::uninit())).cast::<Called>();
	unsafe { p.write(Called(std::cell::UnsafeCell::new((n, n)))) };
	let mut kept = Vec::new();
	kept_part(p, &mut kept);
	unsafe { p.write(Called(std::cell::UnsafeCell::new((0, 0)))) };
	unsafe { (*kept[0]).0 }
}

/// A fresh allocation written whole in each iteration, read after through the pointer an
/// earlier iteration took into it.
pub fn kept_across_writes(data: &[u16]) -> u16 {
	let p = Box::into_raw(Box::new(std::mem::MaybeUninit::<Looped>::uninit())).cast::<Looped>();
	let mut kept: *mut (u16, u16) = std::ptr::null_mut();
	let mut sum = 0;
	for &n in data {
		unsafe { p.write(Looped(std::cell::UnsafeCell::new((n, n)))) };
		if !kept.is_null() {
			sum += unsafe { (*kept).0 };
		}
		kept = unsafe { (*p).0.get() };
	}
	sum
}

/// A fresh allocation written whole through an alias of its pointer while a pointer into it
/// may still be read.
pub fn written_through_alias(n: u16) -> u16 {
	let p = Box::into_raw(Box::new(std::mem::MaybeUninit::<Aliased>::uninit())).cast::<Aliased>();
	unsafe { p.write(Aliased(std::cell::UnsafeCell::new((n, n)))) };
	let part = unsafe { (*p).0.get() };
	let alias = p;
	unsafe { alias.write(Aliased(std::cell::UnsafeCell::new((0, 0)))) };
	unsafe { (*part).0 }
}

/// A `MaybeUninit` written whole again while a pointer into it may still be read.
pub fn uninit_rewritten(n: u16) -> u16 {
	let mut slot = std::mem::MaybeUninit::<Rewritten>::uninit();
	slot.write(Rewritten(std::cell::UnsafeCell::new((n, n))));
	let part = unsafe { slot.assume_init_ref().0.get() };
	slot.write(Rewritten(std::cell::UnsafeCell::new((0, 0))));
	unsafe { (*part).0 }
}

pub struct Loaded(pub std::cell::UnsafeCell<(u16, u16)>);

/// A fresh allocation written whole again through a pointer to it read out of another
/// allocation, while a pointer into it may still be read.
pub fn written_through_loaded(n: u16) -> u16 {
	let p = Box::into_raw(Box::new(std::mem::MaybeUninit::<Loaded>::uninit())).cast::<Loaded>();
	unsafe { p.write(Loaded(std::cell::UnsafeCell::new((n, n)))) };
	let part = unsafe { (*p).0.get() };
	let container = Box::into_raw(Box::new(p.cast_const()));
	unsafe {
		container
			.read()
			.cast_mut()
			.write(Loaded(std::cell::UnsafeCell::new((0, 0))))
	};
	unsafe { (*part).0 }
}

/// A collection of the crate's own, which std's tuple impls extend by its `extend_one`.
#[derive(Default)]
pub struct Tally(pub u8);

impl Extend<u8> for Tally {
	fn extend<I: IntoIterator<Item = u8>>(&mut self, iter: I) {
		for b in iter {
			self.0 = self.0.wrapping_add(b);
		}
	}
}

/// A tuple of collections, one of them the crate's own.
pub fn tallied(data: &[u8]) -> (Vec<u8>, Tally) {
	data.iter().map(|&b| (b, b)).collect()
}

/// A 1-tuple of the crate's own collection, which std's impl extends by its `extend`.
pub fn tallied_alone(data: &[u8]) -> (Tally,) {
	data.iter().map(|&b| (b,)).collect()
}
