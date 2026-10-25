//! Items rust2-ffi cannot export: each is a compile error at its span.

use std::collections::HashMap;

/// A generic type.
pub struct Wrap<T>(pub T);

/// A tuple return.
pub fn pair() -> (u8, u8) {
	(1, 2)
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
