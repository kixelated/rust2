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
