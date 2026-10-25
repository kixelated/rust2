//! Each construct rust2-ffi exports, for `tests/api.c` to drive through the C ABI.

/// A point: a record, since every field is `pub`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
	pub x: i32,
	pub y: i32,
}
