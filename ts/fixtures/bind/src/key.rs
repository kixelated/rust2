//! A key bound to TypeScript as a JS number, which compares, hashes, and orders by its own
//! impls where `===` would tell apart values it finds equal.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// A byte that compares, hashes, and orders by its low four bits alone.
#[derive(Clone, Copy)]
pub struct Nibble(u8);

impl Nibble {
	pub fn new(byte: u8) -> Self {
		Self(byte)
	}

	/// The whole byte, which tells which of equal keys a collection kept.
	pub fn get(self) -> u8 {
		self.0
	}
}

impl PartialEq for Nibble {
	fn eq(&self, other: &Self) -> bool {
		self.0 & 0xf == other.0 & 0xf
	}
}

impl Eq for Nibble {}

impl Hash for Nibble {
	fn hash<H: Hasher>(&self, state: &mut H) {
		(self.0 & 0xf).hash(state);
	}
}

impl PartialOrd for Nibble {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Nibble {
	fn cmp(&self, other: &Self) -> Ordering {
		(self.0 & 0xf).cmp(&(other.0 & 0xf))
	}
}
