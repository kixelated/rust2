//! Each construct rust2-ffi exports, for `tests/api.c` to drive through the C ABI.

use std::borrow::Cow;
use std::fmt;
use std::ops::{Bound, Range, RangeBounds};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::time::Duration;

use bytes::Bytes;

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
}

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
pub struct Token {
	pub id: u32,
}

impl Token {
	pub fn new(id: u32) -> Self {
		Self { id }
	}
}

impl Drop for Token {
	fn drop(&mut self) {
		DROPPED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
