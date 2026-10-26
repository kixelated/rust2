//! Another crate's items, which the api fixture re-exports and names in its own API.

/// A size: a record, though another crate defines it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
	pub width: u32,
	pub height: u32,
}

/// An area: an object, though another crate defines it.
#[derive(Clone, Debug)]
pub struct Area {
	size: Size,
}

impl Area {
	pub fn new(size: Size) -> Self {
		Self { size }
	}

	pub fn size(&self) -> Size {
		self.size
	}

	/// Its width times its height.
	pub fn pixels(&self) -> u64 {
		u64::from(self.size.width) * u64::from(self.size.height)
	}
}
