//! Immutable bytes that clones and slices share, read through `unsafe`: a dependency's
//! type that a consumer binds rather than translates.

use std::ops::{Bound, Deref, RangeBounds};
use std::sync::Arc;

/// Bytes that clones and slices share.
#[derive(Clone)]
pub struct Blob {
	data: Arc<[u8]>,
	start: usize,
	len: usize,
}

impl Blob {
	/// No bytes.
	pub fn new() -> Self {
		Self::copy_from_slice(&[])
	}

	/// Bytes that live for the program.
	pub fn from_static(data: &'static [u8]) -> Self {
		Self::copy_from_slice(data)
	}

	/// A copy of `data`.
	pub fn copy_from_slice(data: &[u8]) -> Self {
		Self {
			data: data.into(),
			start: 0,
			len: data.len(),
		}
	}

	/// The number of bytes.
	pub fn len(&self) -> usize {
		self.len
	}

	/// Whether there are no bytes.
	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// The bytes in `range`, sharing these, panicking out of order or out of bounds.
	pub fn slice(&self, range: impl RangeBounds<usize>) -> Self {
		let begin = match range.start_bound() {
			Bound::Included(&n) => n,
			Bound::Excluded(&n) => n + 1,
			Bound::Unbounded => 0,
		};
		let end = match range.end_bound() {
			Bound::Included(&n) => n + 1,
			Bound::Excluded(&n) => n,
			Bound::Unbounded => self.len,
		};
		assert!(
			begin <= end,
			"range start must not be greater than end: {begin} <= {end}"
		);
		assert!(end <= self.len, "range end out of bounds: {end} <= {}", self.len);
		Self {
			data: self.data.clone(),
			start: self.start + begin,
			len: end - begin,
		}
	}
}

impl Default for Blob {
	fn default() -> Self {
		Self::new()
	}
}

impl From<Vec<u8>> for Blob {
	fn from(data: Vec<u8>) -> Self {
		Self::copy_from_slice(&data)
	}
}

impl Deref for Blob {
	type Target = [u8];

	fn deref(&self) -> &[u8] {
		// SAFETY: `start..start + len` is within `data`, which `slice` checked.
		unsafe { self.data.get_unchecked(self.start..self.start + self.len) }
	}
}

impl PartialEq for Blob {
	fn eq(&self, other: &Self) -> bool {
		**self == **other
	}
}

/// Where bytes come from: a trait, which a binding can't stand in for.
pub trait Source {
	/// The next bytes.
	fn next(&mut self) -> Blob;
}

/// A `Blob` of no bytes.
pub fn empty() -> Blob {
	Blob::new()
}
