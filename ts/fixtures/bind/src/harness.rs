//! The entry point both sides of the differential test run.

use std::sync::Arc;

use bytes::Bytes;

use crate::raw::{Tally, checksum, skip};

/// Split the input as `Bytes`, compare and hash the parts, skip its leading zeros, and
/// count tallies down as they drop. A first byte of 1 slices out of order, which panics.
pub fn run(input: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	let bytes = Bytes::copy_from_slice(input);
	let mid = bytes.len() / 2;
	let head = bytes.slice(..mid);
	let tail = bytes.slice(mid..);
	let inner = bytes.slice(1.min(mid)..mid);
	out.push(head.len() as u8);
	out.push(inner.len() as u8);
	out.extend_from_slice(&tail);
	out.push((head == tail) as u8);
	out.push((head.clone() == head) as u8);
	out.push((Bytes::from(input.to_vec()) == bytes) as u8);
	out.push(bytes.is_empty() as u8);
	let sum = checksum(&tail);
	for shift in [0, 8, 16, 24] {
		out.push((sum >> shift) as u8);
	}
	let mut at = 0;
	skip(input, &mut at);
	out.push(at as u8);
	if input.first() == Some(&1) {
		let _ = bytes.slice(mid..mid.saturating_sub(1));
	}

	let count = Arc::new(input.len() as u32);
	let a = Tally::new(&count);
	let b = Tally::new(&count);
	out.push(Arc::strong_count(&count) as u8);
	drop(a);
	out.push(Arc::strong_count(&count) as u8);
	out.push(b.get() as u8);
	let held = Some(b);
	if input.first() == Some(&2) {
		drop(held);
	}
	out.push(Arc::strong_count(&count) as u8);
	out
}

/// A `Held` of `None`, which `wrong.toml`'s binding would make `undefined`, as the outer
/// `None` is.
pub fn held() -> bool {
	Some(crate::raw::Held::new(None::<u8>)).is_some()
}

/// What `refused.toml`'s bindings can't stand in for, which `run` never reaches.
pub fn refused(count: &Arc<u32>) -> u8 {
	let mut bytes = Bytes::new();
	replace(&mut bytes, Bytes::from_static(b"replaced"));
	let held = crate::raw::Held::new(Tally::new(count));
	drop(held);
	let values = crate::raw::value::<1>() + crate::raw::value::<2>();
	crate::raw::zero::<u8>() + bytes.len() as u8 + values as u8
}

fn replace(target: &mut Bytes, value: Bytes) {
	*target = value;
}
