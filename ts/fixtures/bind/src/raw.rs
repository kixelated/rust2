//! What the crate does through raw pointers, which `bind/` stands in for.

use std::sync::Arc;

/// A module of the function's name, as `rand::rng` has, which the binding's path must not
/// resolve to.
mod checksum {
	pub(super) fn step(sum: u32, byte: u8) -> u32 {
		sum.wrapping_mul(31).wrapping_add(byte as u32)
	}
}

/// A hash of the bytes, read through a raw pointer.
pub fn checksum(bytes: &[u8]) -> u32 {
	let ptr = bytes.as_ptr();
	let mut sum = 0u32;
	for i in 0..bytes.len() {
		// SAFETY: `i` is in bounds.
		let byte = unsafe { *ptr.add(i) };
		sum = checksum::step(sum, byte);
	}
	sum
}

/// A count held by a raw pointer to its `Arc`, which drops it where Rust drops the tally.
pub struct Tally {
	count: *const u32,
}

impl Tally {
	pub fn new(count: &Arc<u32>) -> Self {
		Self {
			count: Arc::into_raw(count.clone()),
		}
	}

	pub fn get(&self) -> u32 {
		// SAFETY: the tally holds the `Arc` it points into.
		unsafe { *self.count }
	}
}

impl Drop for Tally {
	fn drop(&mut self) {
		// SAFETY: `new` made the pointer with `Arc::into_raw`, once.
		drop(unsafe { Arc::from_raw(self.count) });
	}
}

/// A default made from its type alone, which a generic TS function can't make.
pub fn zero<T: Default>() -> T {
	T::default()
}

/// A value it holds, and drops where Rust drops it.
pub struct Held<T>(T);

impl<T> Held<T> {
	pub fn new(value: T) -> Self {
		Self(value)
	}
}
