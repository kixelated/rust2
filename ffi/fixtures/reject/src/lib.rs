//! Items rust2-ffi cannot export: each is a compile error at its span.

use std::collections::HashMap;

/// A generic type.
pub struct Wrap<T>(pub T);

/// A tuple inside an `Option`: only a returned tuple crosses, as out pointers.
pub fn pair() -> Option<(u8, u8)> {
	Some((1, 2))
}

/// Another crate's type, which has no mapping.
pub fn map() -> HashMap<u8, u8> {
	HashMap::new()
}

/// A type parameter no rule picks a type for.
pub fn generic<T: Copy>(value: T) -> T {
	value
}

/// A future that borrows its argument, from a function that runs when called: the export
/// would have to keep the argument alive past the call.
#[allow(clippy::manual_async_fn, reason = "the construct under test")]
pub fn borrowing(name: &str) -> impl Future<Output = usize> + '_ {
	async move { name.len() }
}

/// An option inside an option, through a `Box`: C has one `None` per option.
pub fn nested(value: Option<Box<Option<u32>>>) -> bool {
	value.is_some()
}

/// An object, since its field is private.
pub struct Opaque(u8);

/// A handle lent inside a `Box`: it would cross by value, moving it out of the caller's
/// handle for a call that only reads it.
pub fn peek(value: &Box<Opaque>) -> u8 {
	value.0
}

/// A static.
pub static COUNT: u8 = 0;

/// A macro.
#[macro_export]
macro_rules! noop {
	() => {};
}

/// Left out by the config, so it refuses nothing itself, but a signature naming it is
/// refused.
pub struct Excluded;

pub fn excluded() -> Excluded {
	Excluded
}

pub mod audio_encoder {
	/// `reject_audio_encoder_finish` in C ...
	pub fn finish() {}
}

pub mod audio {
	/// ... and so is this: the clash is refused at both.
	pub fn encoder_finish() {}
}

/// A range of what is not an integer.
pub fn letters(range: std::ops::Range<char>) -> usize {
	range.count()
}

/// A text that borrows: only its `'static` form crosses, held by a handle.
pub struct Text<'a>(&'a str);

impl<'a> Text<'a> {
	/// Its `'static` form would borrow what C lends for the call past it.
	pub fn new(text: &'a str) -> Self {
		Self(text)
	}

	/// A text that borrows its handle, which a new handle would outlive.
	pub fn view(&self) -> Text<'_> {
		Text(self.0)
	}
}

/// A borrow `text` could keep, since it lives as long as what it borrows.
pub fn retext<'a>(text: &mut Text<'a>, with: &'a str) {
	text.0 = with;
}

/// A borrowed return that is not `Clone`, so it has no copy to return.
pub fn first(opaque: &Opaque) -> Option<&Opaque> {
	Some(opaque)
}

impl Opaque {
	pub fn new() -> Self {
		Self(0)
	}
}

impl<'a> Text<'a> {
	pub fn empty() -> Self {
		Self("")
	}

	/// `'b` outlives the handle's `'a`, so it is `'static` too.
	pub fn replace<'b: 'a>(&mut self, text: &'b str) {
		self.0 = text;
	}

	/// What a future resolves to borrows its handle, as `view` does.
	pub async fn later(&self) -> Text<'_> {
		Text(self.0)
	}
}

/// An optional borrow: C's value is lifted into an owned one, lent only as the parameter
/// itself.
pub fn either(text: Option<&str>) -> usize {
	text.map_or(0, str::len)
}

/// An object no export returns but its own method, so C never holds one.
pub struct Lease(u8);

impl Lease {
	pub fn split(&mut self) -> Lease {
		Lease(self.0)
	}
}

/// `reject_size` in C, as another export's `Size` is: one prefix serves every export.
pub struct Size {
	pub bytes: u64,
}

/// Another export's type, which this one names.
pub fn measure(size: dep::Size) -> dep::Area {
	dep::Area::new(size)
}

/// A generic type, which exports at the instances its exports name.
pub struct Cell<T>(T);

impl<T> Cell<T> {
	pub fn new(value: T) -> Self {
		Self(value)
	}
}

impl Cell<u8> {
	/// An instance holding an instance of its own type, which could name ever deeper ones.
	pub fn nest(self) -> Cell<Cell<u8>> {
		Cell(self)
	}
}

/// Two generic types whose methods each wrap the other's instance in their own: inferring
/// from `Ping<u8>` names `Pong<Ping<u8>>`, whose `ping` names `Ping<Pong<Ping<u8>>>`, which
/// holds a `Ping` inside a `Ping` and ends inference there.
pub struct Ping<T>(T);

pub struct Pong<T>(T);

impl<T> Ping<T> {
	pub fn pong(self) -> Pong<Ping<T>> {
		Pong(self)
	}
}

impl<T> Pong<T> {
	pub fn ping(self) -> Ping<Pong<T>> {
		Ping(self)
	}
}

pub fn ping(value: u8) -> Ping<u8> {
	Ping(value)
}

/// A generic type whose method wraps its argument in a tuple: `Grow<(u32,)>` is refused,
/// as a tuple is no type argument, so inference names nothing through its `grow`.
pub struct Grow<T>(T);

impl<T> Grow<T> {
	pub fn grow(self) -> Grow<(T,)> {
		Grow((self.0,))
	}
}

pub fn grow(value: u32) -> Grow<u32> {
	Grow(value)
}

/// A record whose instance at a tuple is refused before its fields map, so `Core<u8>`,
/// which only those fields name, is neither inferred nor needed.
pub struct Shell<T> {
	pub core: Core<u8>,
	pub tag: T,
}

pub struct Core<T> {
	pub value: T,
}

pub fn shell() -> Shell<(u32,)> {
	Shell {
		core: Core { value: 1 },
		tag: (2,),
	}
}

/// A const generic function whose bound names an instance nothing else does: refused at
/// its const parameter, before any candidate for the other is mapped.
pub struct Probe<T>(T);

pub fn probe<const N: usize>(x: impl Into<Probe<u8>>) {
	let _ = x;
}

/// An instance the config excludes.
pub fn excluded_cell(cell: Cell<u16>) -> u16 {
	cell.0
}

/// An instance whose type argument is no value or handle, so it cannot name it.
pub fn listed(cell: Cell<Vec<String>>) -> usize {
	cell.0.len()
}

/// A const-generic type, whose const argument the IR cannot carry.
pub struct Fixed<const N: usize>(pub u8);

/// An instance of it, which a signature names.
pub fn fixed() -> Fixed<2> {
	Fixed(2)
}

/// Another crate's item that is no export, so nothing names it.
pub use std::collections::HashSet;

/// A borrowed waker: one crosses only into Rust, which keeps it.
pub fn lend(waker: &std::task::Waker) {
	waker.wake_by_ref();
}

/// A field and a method of one name, which claim one C name until `field@` renames one.
#[non_exhaustive]
pub struct Limit {
	pub max: u32,
}

impl Limit {
	pub fn new(max: u32) -> Self {
		Self { max }
	}

	pub fn max(&self) -> u32 {
		self.max.max(1)
	}

	/// The name of `max`'s setter too.
	pub fn set_max(&mut self, max: u32) {
		self.max = max;
	}
}

/// A field holding a `'static` borrow, which a setter cannot take from C's borrow for
/// the call.
#[non_exhaustive]
pub struct Tag {
	pub label: &'static str,
}

impl Tag {
	pub fn new() -> Self {
		Self { label: "tag" }
	}
}

/// A closure kept past the call, as a `'static` bound lets it be.
pub fn spawn(task: impl FnOnce() + Send + 'static) {
	std::thread::spawn(task);
}

/// A closure the future holds past the call.
pub async fn later(task: impl FnOnce()) {
	task()
}

/// A closure bound by more than C's function can meet.
pub fn twice(f: impl Fn() + Clone) {
	f.clone()();
	f()
}

/// One closure type for two parameters, which C's two functions cannot be.
pub fn both<F: Fn()>(a: F, b: F) {
	a();
	b()
}

/// An object lent only for as long as it borrows, which a handle's `'static` outlives.
pub fn read(f: impl FnOnce(&Text<'_>)) {
	f(&Text(""))
}

/// Words C would lend inside an array, which C lends nothing inside.
pub fn count_words(words: Vec<&str>) -> usize {
	words.len()
}

/// Words C would lend inside an `Option` inside an array.
pub fn count_some(words: Vec<Option<&str>>) -> usize {
	words.iter().flatten().count()
}

/// A slice of references C cannot lend, as a `Vec` of them.
pub fn count_slice(words: &[&str]) -> usize {
	words.len()
}

/// A slice of handles: lifting each would move it out of its caller's handle.
pub fn peek_all(values: &[Opaque]) -> usize {
	values.len()
}

/// A borrowed slice whose elements have no clone to return.
pub fn none_opaque() -> &'static [Opaque] {
	&[]
}

/// A range is a value, not an iterator to collect: one that maps to no range is refused.
pub fn alphabet() -> std::ops::Range<char> {
	'a'..'z'
}

/// A range with no end, which collecting would never finish.
pub fn naturals() -> std::ops::RangeFrom<u64> {
	0..
}

/// An iterator of handles with no clone to return.
pub fn opaques() -> std::slice::Iter<'static, Opaque> {
	[].iter()
}

/// An iterator inside a `Result`, which crosses only as a whole return.
pub fn tried() -> Result<std::vec::IntoIter<u8>, String> {
	Ok(Vec::new().into_iter())
}

/// An iterator a future resolves to.
pub async fn eventually() -> std::vec::IntoIter<u8> {
	Vec::new().into_iter()
}

/// A callback returning an iterator, which C would have to make.
pub trait Source: Send + Sync {
	fn items(&self) -> std::vec::IntoIter<u8>;
}
