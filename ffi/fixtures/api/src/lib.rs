//! Each construct rust2-ffi exports, for `tests/api.c` to drive through the C ABI.

use std::borrow::Cow;
use std::fmt;
use std::ops::{Bound, Range, RangeBounds};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::time::Duration;

use bytes::Bytes;
// Another export's type, which this names rather than exports again.
pub use dep::Size;

/// The largest value a [`Counter`] reaches.
pub const LIMIT: u64 = 1000;

/// A point: a record, since every field is `pub`. Its docs reach the header, where a
/// comment such as `/* this */` must not close the header's own.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
	pub x: i32,
	pub y: i32,
}

impl Point {
	/// The point moved by `dx` and `dy`.
	pub fn offset(&self, dx: i32, dy: i32) -> Point {
		Point {
			x: self.x + dx,
			y: self.y + dy,
		}
	}
}

/// A record that owns memory: its `free` releases the name.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
	pub name: String,
	pub at: Option<Point>,
	pub ttl: Option<Duration>,
}

/// A note whose text may be present but empty, which crosses as itself, not as none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {
	pub text: Option<String>,
}

/// Whether `note` has text, even empty.
pub fn has_text(note: Note) -> bool {
	note.text.is_some()
}

/// A plain enumeration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
	Circle = 1,
	Square = 4,
}

/// An enum with data.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
	Moved(Point),
	Named { name: String },
	Done,
}

/// The error a [`Counter`] returns.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
	Underflow { by: u64 },
	Overflow,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::Underflow { by } => write!(f, "underflow by {by}"),
			Error::Overflow => write!(f, "overflow"),
		}
	}
}

/// A counter: an object, since its field is private. Clones share nothing.
#[derive(Clone, Debug, Default)]
pub struct Counter {
	value: u64,
	name: String,
}

impl Counter {
	pub fn new(name: &str, start: u64) -> Self {
		Self {
			value: start,
			name: name.to_string(),
		}
	}

	pub fn get(&self) -> u64 {
		self.value
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn add(&mut self, n: u64) -> Result<u64, Error> {
		match self.value.checked_add(n) {
			Some(v) if v <= LIMIT => {
				self.value = v;
				Ok(v)
			}
			_ => Err(Error::Overflow),
		}
	}

	pub fn sub(&mut self, n: u64) -> Result<u64, Error> {
		match self.value.checked_sub(n) {
			Some(v) => {
				self.value = v;
				Ok(v)
			}
			None => Err(Error::Underflow { by: n - self.value }),
		}
	}

	/// Whether this is the counter [`Registry::edit`] lends, by its address, which a handle
	/// lent for the call keeps.
	pub fn edited(&self) -> bool {
		std::ptr::from_ref(self) as usize == EDITING.load(std::sync::atomic::Ordering::Relaxed)
	}

	/// Consumes the counter.
	pub fn into_label(self, at: Option<Point>) -> Label {
		Label {
			name: self.name,
			at,
			ttl: None,
		}
	}

	pub fn rename(&mut self, name: impl Into<String>) {
		self.name = name.into();
	}

	/// Consumes the counter into two, the second holding what is above `at`: a tuple
	/// returns through one out pointer per element.
	pub fn split(self, at: u64) -> (Counter, Counter) {
		let above = self.value.saturating_sub(at);
		let below = Counter::new(&self.name, self.value - above);
		(below, Counter::new(&format!("{}+", self.name), above))
	}
}

/// An impl lifetime its type does not take is the call's, as a method's own is.
impl<'a> Counter {
	/// Whether the counter is named `name`, which borrows C's text for the call.
	pub fn is(&self, name: &'a str) -> bool {
		self.name == name
	}
}

/// A tuple of one: its out pointer holds the counter, not the tuple.
pub fn only(counter: Counter) -> (Counter,) {
	(counter,)
}

/// `n` narrowed to each width, a tuple longer than any a language may name.
pub fn widths(n: u64) -> (u8, i16, u32, u64) {
	(n as u8, n as i16, n as u32, n)
}

/// The quotient and remainder, failing on a zero divisor.
pub fn divide(a: u64, b: u64) -> Result<(u64, u64), Error> {
	match b {
		0 => Err(Error::Overflow),
		b => Ok((a / b, a % b)),
	}
}

/// Holds counters and lends them: each borrowed return is a new handle to a clone.
#[derive(Default)]
pub struct Registry {
	pub first: Counter,
	last: Option<Counter>,
}

impl Registry {
	pub fn new(first: Counter) -> Self {
		Self { first, last: None }
	}

	pub fn last(&self) -> Option<&Counter> {
		self.last.as_ref()
	}

	pub fn push(&mut self, counter: Counter) {
		self.last = Some(counter);
	}

	/// The first counter lent to `edit` for the call, returning what `edit` does: a closure
	/// called only during the call, which C lends as a function.
	pub fn edit(&mut self, edit: impl FnOnce(&mut Counter) -> bool) -> bool {
		EDITING.store(
			std::ptr::from_ref(&self.first) as usize,
			std::sync::atomic::Ordering::Relaxed,
		);
		edit(&mut self.first)
	}
}

/// The address of the counter [`Registry::edit`] lends.
static EDITING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A name that may borrow its text: its only generic parameter is a lifetime, so a
/// handle holds `Name<'static>`, which borrows nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Name<'a>(Cow<'a, str>);

impl Name<'_> {
	pub fn owned(text: String) -> Name<'static> {
		Name(Cow::Owned(text))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	/// `other` after this name, as a name that borrows neither.
	pub fn join(&self, other: &Name<'_>) -> Name<'static> {
		Name(Cow::Owned(format!("{}/{}", self.0, other.0)))
	}

	/// Takes a parameter that must outlive the call, so a `String`, not a borrow of C's.
	pub fn rename(&mut self, text: impl AsRef<str> + Send + 'static) {
		self.0 = Cow::Owned(text.as_ref().to_string());
	}
}

/// The longer name: it takes only the lifetime of the names given, `'static` in a handle.
pub fn longer<'a>(a: Name<'a>, b: &Name<'a>) -> Name<'a> {
	match b.0.len() > a.0.len() {
		true => b.clone(),
		false => a,
	}
}

/// A name of no text. A lifetime only the return takes is the caller's to pick, so the
/// handle's `'static`.
pub fn unnamed<'a>() -> Name<'a> {
	Name(Cow::Borrowed(""))
}

/// The sum of a range.
pub fn total(range: Range<u64>) -> u64 {
	range.sum()
}

/// A range, widened by `by` at each end.
pub fn widen(range: Range<u64>, by: u64) -> Range<u64> {
	range.start.saturating_sub(by)..range.end + by
}

/// How many of `0..10` a range takes, in any of the forms `RangeBounds` takes.
pub fn count(range: impl RangeBounds<u64>) -> usize {
	(0..10).filter(|n| range.contains(n)).count()
}

/// The end a limit puts on a range: none for 0.
pub fn end(limit: u64) -> Bound<u64> {
	match limit {
		0 => Bound::Unbounded,
		limit => Bound::Excluded(limit),
	}
}

/// A record of a range and a bound, which C holds after the structs they are.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
	pub range: Range<u64>,
	pub end: Bound<u64>,
}

impl Window {
	pub fn contains(&self, n: u64) -> bool {
		self.range.contains(&n) && (Bound::Unbounded, self.end).contains(&n)
	}
}

/// A one-slot channel: its receive is a future that waits for a send.
#[derive(Clone, Default)]
pub struct Channel {
	state: Arc<Mutex<(Option<Bytes>, Option<Waker>)>>,
}

impl Channel {
	pub fn new() -> Self {
		Self::default()
	}

	/// Drop any payload not yet received. Exclusive, so it cannot run while a receive
	/// borrows the channel.
	pub fn clear(&mut self) {
		self.state.lock().unwrap().0 = None;
	}

	pub fn send(&self, payload: Bytes) {
		let mut state = self.state.lock().unwrap();
		state.0 = Some(payload);
		if let Some(waker) = state.1.take() {
			waker.wake();
		}
	}

	/// The next payload sent, waiting for it.
	pub async fn recv(&self) -> Bytes {
		std::future::poll_fn(|cx| {
			let mut state = self.state.lock().unwrap();
			match state.0.take() {
				Some(payload) => Poll::Ready(payload),
				None => {
					state.1 = Some(cx.waker().clone());
					Poll::Pending
				}
			}
		})
		.await
	}

	/// Wakes `waker` on the next send, keeping it until then, as a driver's poll keeps the
	/// waker it is given.
	pub fn notify(&self, waker: Waker) {
		self.state.lock().unwrap().1 = Some(waker);
	}

	/// Sends at once, as a function that is not `async` runs when called, and returns a
	/// future that borrows nothing.
	pub fn send_later(&self, payload: Bytes) -> impl Future<Output = ()> + use<> {
		self.send(payload);
		std::future::ready(())
	}

	/// `recv`'s poll, which does not map: left out, since `recv` exports as a future.
	pub fn poll_recv(&self, cx: &mut std::task::Context<'_>) -> Poll<Bytes> {
		let mut state = self.state.lock().unwrap();
		match state.0.take() {
			Some(payload) => Poll::Ready(payload),
			None => {
				state.1 = Some(cx.waker().clone());
				Poll::Pending
			}
		}
	}

	/// The next payload and its length: a future's tuple is written through its poll's
	/// out pointers.
	pub async fn recv_sized(&self) -> (Bytes, usize) {
		let payload = self.recv().await;
		let len = payload.len();
		(payload, len)
	}

	/// Fails once the payload is empty.
	pub async fn recv_nonempty(&self) -> Result<Bytes, Error> {
		match self.recv().await {
			payload if payload.is_empty() => Err(Error::Underflow { by: 1 }),
			payload => Ok(payload),
		}
	}
}

/// `value`, on its second poll: the first wakes its waker before it returns pending, so
/// the wake lands before the poller waits for one.
pub async fn yielded(value: u32) -> u32 {
	let mut woken = false;
	std::future::poll_fn(|cx| match std::mem::replace(&mut woken, true) {
		true => Poll::Ready(()),
		false => {
			cx.waker().wake_by_ref();
			Poll::Pending
		}
	})
	.await;
	value
}

/// A time: an object, since its field is private, but `Copy`, so passing it by value
/// copies it rather than moving it out of its handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stamp {
	millis: u64,
}

impl Stamp {
	pub fn from_millis(millis: u64) -> Self {
		Self { millis }
	}

	pub fn secs(self) -> u64 {
		self.millis / 1000
	}

	pub fn later(self, other: Stamp) -> bool {
		self.millis > other.millis
	}
}

/// A message: an object, since one field is private, whose `pub` fields read as getters.
#[derive(Clone, Debug)]
pub struct Message {
	pub stamp: Option<Stamp>,
	pub payload: Bytes,
	sent: bool,
}

impl Message {
	pub fn new(stamp: Option<Stamp>, payload: Bytes) -> Self {
		Self {
			stamp,
			payload,
			sent: false,
		}
	}

	pub fn sent(&self) -> bool {
		self.sent
	}
}

/// A sink: a callback interface, implemented in C.
pub trait Log: Send + Sync {
	/// Whether the line was kept.
	fn log(&self, level: u8, line: &str) -> bool;
}

/// Each line to `log` as a type parameter takes it, returning how many it kept.
pub fn emit(log: impl Log, lines: Vec<u8>) -> u32 {
	lines.iter().filter(|&&n| log.log(n, &format!("line {n}"))).count() as u32
}

/// Each line to `keep` on another thread, returning how many it kept: a lent closure may
/// run on any thread until the call returns.
pub fn filter(lines: Vec<u8>, mut keep: impl FnMut(u8, &str) -> bool + Send) -> u32 {
	std::thread::scope(|scope| {
		let kept = scope.spawn(move || lines.iter().filter(|&&n| keep(n, &format!("line {n}"))).count());
		kept.join().expect("the thread finishes") as u32
	})
}

/// Holds a foreign implementation as a trait object, dropping it with itself.
pub struct Logger {
	sink: Box<dyn Log>,
}

impl Logger {
	pub fn new(sink: Box<dyn Log>) -> Self {
		Self { sink }
	}

	pub fn write(&self, line: &str) -> bool {
		self.sink.log(0, line)
	}
}

/// A list: every field is `pub`, but it holds itself, which C cannot hold in place, so it
/// is an object.
#[derive(Debug)]
pub struct Node {
	pub value: u64,
	pub next: Option<Box<Node>>,
}

impl Node {
	pub fn new(value: u64, next: Option<Box<Node>>) -> Self {
		Self { value, next }
	}

	/// Prepend `value`.
	pub fn push(&mut self, value: u64) {
		let rest = std::mem::replace(self, Node { value, next: None });
		self.next = Some(Box::new(rest));
	}

	pub fn sum(&self) -> u64 {
		self.value + self.next.as_ref().map_or(0, |next| next.sum())
	}
}

/// A tally: `Send` but not `Sync`, so even `&self` borrows it exclusively, and comparing
/// a handle with itself is one borrow.
#[derive(Debug, Default, PartialEq)]
pub struct Tally {
	count: std::cell::Cell<u32>,
}

impl Tally {
	pub fn new() -> Self {
		Self::default()
	}

	/// Counts through `&self`, which only an exclusive borrow makes sound across threads.
	pub fn bump(&self) -> u32 {
		self.count.set(self.count.get() + 1);
		self.count.get()
	}
}

static DROPPED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A token: every field is `pub`, but its `Drop` must run when C frees it, so it is an
/// object rather than a record.
#[derive(Clone)]
pub struct Token {
	pub id: u32,
}

impl Token {
	pub fn new(id: u32) -> Self {
		Self { id }
	}

	/// The token twice, borrowed: C gets a clone of each, which the array frees.
	pub fn twice(&self) -> Vec<&Token> {
		vec![self, self]
	}
}

impl Drop for Token {
	fn drop(&mut self) {
		DROPPED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	}
}

/// Retry settings: each field is unset until C sets it, and the method of its name
/// resolves it, as Rust's accessors do. Not exhaustive, so an object with getters.
#[non_exhaustive]
pub struct Retry {
	pub limit: Option<u32>,
	pub delay: Option<u32>,
}

impl Retry {
	pub fn new(limit: Option<u32>) -> Self {
		Self { limit, delay: None }
	}

	pub fn limit(&self) -> u32 {
		self.limit.unwrap_or(3)
	}

	pub fn delay(&self) -> u32 {
		self.delay.unwrap_or(100)
	}
}

/// How many tokens have dropped.
pub fn dropped() -> u32 {
	DROPPED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Both tallies bumped: C may pass one handle twice, which Rust allows of `&`.
pub fn bump_both(a: &Tally, b: &Tally) -> u32 {
	a.bump() + b.bump()
}

/// What `by` returns, plus `by_data`: a closure taking nothing, whose data C names apart.
pub fn offset(by: impl Fn() -> u32, by_data: u32) -> u32 {
	by() + by_data
}

/// A position, which C reaches only as [`walk`] lends it.
pub struct Cursor {
	at: u64,
}

impl Cursor {
	pub fn at(&self) -> u64 {
		self.at
	}
}

/// A cursor at each of `0..steps` lent shared to `visit`, summing what it returns.
pub fn walk(steps: u64, mut visit: impl FnMut(&Cursor) -> u64) -> u64 {
	(0..steps).map(|at| visit(&Cursor { at })).sum()
}

/// The tally lent shared to `peek` for the call, on one thread, as it is not `Sync`.
pub fn peek(tally: &Tally, peek: impl FnOnce(&Tally) -> u32) -> u32 {
	peek(tally)
}

/// The length of a payload, borrowed for the call.
pub fn len(payload: &[u8]) -> usize {
	payload.len()
}

pub fn greet(name: impl Into<String>, shape: Shape) -> String {
	format!("hello {}, a {shape:?}", name.into())
}

pub fn describe(event: &Event) -> String {
	format!("{event:?}")
}

/// Two values: a generic record, which exports at each instance it infers. Its second
/// parameter defaults to its first, as an instance's name leaves it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pair<L, R = L> {
	pub left: L,
	pub right: R,
}

impl<L: Copy, R: Copy> Pair<L, R> {
	/// The pair swapped: another instance, unless both are of one type.
	pub fn swap(&self) -> Pair<R, L> {
		Pair {
			left: self.right,
			right: self.left,
		}
	}

	/// Whether the sides are equal: a method of only the instances whose sides compare.
	pub fn even(&self) -> bool
	where
		L: PartialEq<R>,
	{
		self.left == self.right
	}
}

/// An impl of one instance, whose methods no other instance has, and which infers it.
impl Pair<u32> {
	pub fn sum(&self) -> u32 {
		self.left + self.right
	}

	/// The left value as a byte, beside `right`: the one instance that names the result
	/// fills the type parameter, with `dep_fixture::Size`, and names this instance.
	pub fn beside<T: Copy>(&self, right: T) -> Pair<u8, T> {
		Pair {
			left: self.left as u8,
			right,
		}
	}
}

/// A size tagged with a byte: a signature that names an instance, which infers it, and
/// its `swap` the other way around.
pub fn tag(size: Size, tag: u8) -> Pair<Size, u8> {
	Pair { left: size, right: tag }
}

/// A stack: a generic object, at its default of objects, whose handles move in and out.
#[derive(Debug, Default)]
pub struct Stack<T = Counter> {
	items: Vec<T>,
}

impl<T> Stack<T> {
	pub fn new() -> Self {
		Self { items: Vec::new() }
	}

	pub fn push(&mut self, item: T) {
		self.items.push(item);
	}

	pub fn pop(&mut self) -> Option<T> {
		self.items.pop()
	}

	pub fn depth(&self) -> usize {
		self.items.len()
	}

	/// The depth past `from`'s seed: a generic type's generic method.
	pub fn depth_past(&self, from: impl Seed<u64>) -> u64 {
		self.items.len() as u64 + from.seed()
	}
}

/// What a [`Counter`] can start from: generic, so no foreign implementation takes it, and a
/// parameter it bounds exports at each type the exports implement it for, by reference,
/// which also implements it, so C keeps its handle.
pub trait Seed<T> {
	fn seed(&self) -> T;
}

/// A weight, implemented for `Point` and for `&Point` apart, which both choose `&Point`:
/// one instance of a parameter it bounds.
impl Seed<u32> for Point {
	fn seed(&self) -> u32 {
		self.x.unsigned_abs() + self.y.unsigned_abs()
	}
}

impl Seed<u32> for &Point {
	fn seed(&self) -> u32 {
		Seed::<u32>::seed(*self)
	}
}

/// Two generic records whose instances name each other, as `Left<Right<u8>>` and
/// `Right<Left<u8>>` do: each instance is named without the other's name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Left<T> {
	pub value: T,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Right<T> {
	pub value: T,
}

/// The sum of what each holds inside.
pub fn cross(left: Left<Right<u8>>, right: Right<Left<u8>>) -> u8 {
	left.value.value + right.value.value
}

/// A handle made only by an impl of an instance holding another, which nothing else
/// names: inferring the outer instance infers the inner one.
pub struct Outer<T>(T);

impl Outer<Inner<u8>> {
	pub fn new(value: u8) -> Self {
		Self(Inner { value })
	}

	pub fn get(&self) -> u8 {
		self.0.value
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inner<T> {
	pub value: T,
}

/// `from`'s seed, which outlives the call, so a reference to a value C passes cannot be
/// it: each instance takes its value.
pub fn kept(from: impl Seed<u64> + 'static) -> u64 {
	from.seed()
}

/// A generic record only a constant names, which infers its instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boxed<T> {
	pub value: T,
}

pub const BOXED: Boxed<u16> = Boxed { value: 42 };

/// Whether `log` kept a line, boxed: an instance only a function taking a foreign
/// implementation names.
pub fn keeps(log: impl Log) -> Boxed<u8> {
	Boxed {
		value: u8::from(log.log(2, "keeps")),
	}
}

/// `from`'s weight.
pub fn weigh(from: impl Seed<u32>) -> u32 {
	from.seed()
}

impl<S: Seed<u64>> Seed<u64> for &S {
	fn seed(&self) -> u64 {
		(**self).seed()
	}
}

impl Seed<u64> for Counter {
	fn seed(&self) -> u64 {
		self.value
	}
}

impl Seed<u64> for Point {
	fn seed(&self) -> u64 {
		u64::from(self.x.unsigned_abs()) + u64::from(self.y.unsigned_abs())
	}
}

impl Seed<u64> for Size {
	fn seed(&self) -> u64 {
		u64::from(self.width) * u64::from(self.height)
	}
}

/// Each side of `pair` through `f`, summed: a function inferred at each instance of
/// `Pair` whose sides are alike, taking a lent closure of the inferred type.
pub fn apply<T: Copy, F: Fn(T) -> u32>(pair: Pair<T>, f: F) -> u32 {
	f(pair.left) + f(pair.right)
}

/// Each side of `pair` to `log`, returning how many it kept: inferred as `apply` is,
/// taking a foreign implementation.
pub fn report<T: Copy + fmt::Debug, L: Log>(pair: Pair<T>, log: L) -> u32 {
	[pair.left, pair.right]
		.iter()
		.filter(|side| log.log(1, &format!("{side:?}")))
		.count() as u32
}

/// A wrapped value, which only its impl of `Seed<u64>` at `Wrap<u32>` names: each
/// function taking a seed infers that instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrap<T> {
	pub value: T,
}

impl Seed<u64> for Wrap<u32> {
	fn seed(&self) -> u64 {
		u64::from(self.value) * 10
	}
}

/// A second `Point`, whose last segment clashes with the crate root's: an instance named
/// by it takes its module too, and one named by the other keeps the shorter name.
pub mod polar {
	/// A point by its angle in turns and its distance from the origin.
	#[derive(Clone, Copy, Debug, PartialEq)]
	pub struct Point {
		pub turns: u8,
		pub distance: u32,
	}

	impl super::Seed<u64> for Point {
		fn seed(&self) -> u64 {
			u64::from(self.turns) + u64::from(self.distance)
		}
	}
}

impl Counter {
	/// A counter named `name` at `from`'s seed: an `impl Trait` argument, at an instance
	/// for each implementation.
	pub fn seeded(name: &str, from: impl Seed<u64>) -> Counter {
		Counter::new(name, from.seed())
	}
}

/// The seed of `T`'s default and `from`'s: a type parameter no argument names, which the
/// call names, beside an `impl Trait` one.
pub fn seeds<T: Seed<u64> + Default>(from: impl Seed<u64>) -> u64 {
	T::default().seed() + from.seed()
}

/// An area grown by `by` on each side: another export's types, across crates.
pub fn grow(area: &dep::Area, by: u32) -> dep::Area {
	let size = area.size();
	dep::Area::new(Size {
		width: size.width + by,
		height: size.height + by,
	})
}

/// A slot whose method `value` only `Slot<u8>` has, so its path alone names `Slot<u32>`'s
/// field and `Slot<u8>`'s method. Not exhaustive, so an object with getters.
#[non_exhaustive]
pub struct Slot<T> {
	pub value: T,
}

impl<T> Slot<T> {
	pub fn new(value: T) -> Self {
		Self { value }
	}
}

impl Slot<u8> {
	/// The value widened and doubled, apart from the field.
	pub fn value(&self) -> u16 {
		u16::from(self.value) * 2
	}
}

/// A wide slot: a signature that names `Slot<u32>`, which infers it.
pub fn wide(value: u32) -> Slot<u32> {
	Slot::new(value)
}

/// A peer: a record of a socket address and its names, which holds a `Vec`.
#[derive(Clone, Debug, PartialEq)]
pub struct Peer {
	pub addr: std::net::SocketAddr,
	pub names: Vec<String>,
}

impl Peer {
	/// Its names, each borrowed.
	pub fn aliases(&self) -> Vec<&str> {
		self.names.iter().map(String::as_str).collect()
	}

	/// The peer at the next port, also named `name`.
	pub fn next(&self, name: &str) -> Peer {
		let mut addr = self.addr;
		addr.set_port(addr.port() + 1);
		let mut names = self.names.clone();
		names.push(name.to_string());
		Peer { addr, names }
	}
}

/// `url`'s host, if it has one.
pub fn host(url: &url::Url) -> Option<String> {
	url.host_str().map(str::to_string)
}

/// `path` resolved against `base`, or why it does not resolve.
pub fn resolve(base: url::Url, path: &str) -> Result<url::Url, String> {
	base.join(path).map_err(|err| err.to_string())
}

/// Each address's port.
pub fn ports(addrs: Vec<std::net::SocketAddr>) -> Vec<u16> {
	addrs.iter().map(std::net::SocketAddr::port).collect()
}

/// `name` in `dir`.
pub fn file(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
	dir.join(name)
}

/// A path whose bytes are not UTF-8, as a Unix path may be.
pub fn stray() -> std::path::PathBuf {
	std::path::PathBuf::from(<std::ffi::OsString as std::os::unix::ffi::OsStringExt>::from_vec(vec![
		0xff,
	]))
}

/// Each path's parent, if it has one.
pub fn parents(paths: Vec<std::path::PathBuf>) -> Vec<Option<std::path::PathBuf>> {
	paths
		.iter()
		.map(|path| path.parent().map(std::path::Path::to_path_buf))
		.collect()
}

/// The words of `text`.
pub fn words(text: &str) -> Vec<String> {
	text.split_whitespace().map(str::to_string).collect()
}

/// `words` joined by spaces, or `none` for none.
pub fn sentence(words: Option<Vec<String>>) -> String {
	words.map_or_else(|| "none".to_string(), |words| words.join(" "))
}

/// `bytes` in chunks of up to `size`, which is not 0.
pub fn chunks(bytes: &[u8], size: usize) -> Vec<Vec<u8>> {
	bytes.chunks(size).map(<[u8]>::to_vec).collect()
}

/// A counter for each name, at 1.
pub fn counters(names: Vec<String>) -> Vec<Counter> {
	names.iter().map(|name| Counter::new(name, 1)).collect()
}

/// The counters' values summed, none counting 0: each moves in, through an `Option` or a
/// `Box`.
pub fn worth(some: Option<Vec<Counter>>, boxed: Box<Vec<Counter>>) -> u64 {
	some.into_iter().flatten().chain(*boxed).map(|c| c.get()).sum()
}

/// Each row's sum.
pub fn sums(rows: Vec<Vec<u32>>) -> Vec<u32> {
	rows.iter().map(|row| row.iter().sum()).collect()
}

/// The points added up, from anything that iterates them.
pub fn summed(points: impl IntoIterator<Item = Point>) -> Point {
	points.into_iter().fold(Point::default(), |sum, p| sum.offset(p.x, p.y))
}

/// The length of the longest word, naming its iterator.
pub fn longest<I>(words: I) -> usize
where
	I: IntoIterator<Item = String>,
{
	words.into_iter().map(|word| word.len()).max().unwrap_or(0)
}

/// `value` as compact JSON text: a serde bound takes a JSON document.
pub fn encode<T: serde::Serialize>(value: &T) -> String {
	serde_json::to_string(value).expect("a JSON document serializes")
}

/// `doc` under `tag`, as a new document.
pub fn tagged(doc: serde_json::Value, tag: &str) -> serde_json::Value {
	serde_json::json!({ "tag": tag, "doc": doc })
}

/// A site: every field is `pub`, but a URL is a handle C cannot copy, so it is an object
/// whose getter returns a clone.
pub struct Site {
	pub home: url::Url,
}

impl Site {
	pub fn new(home: url::Url) -> Self {
		Self { home }
	}
}
