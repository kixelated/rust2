//! The entry point both sides of the differential test run.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use buf::Blob;

use crate::key::Nibble;
use crate::raw::{Tally, checksum, skip};

/// Split the input as `Blob`, compare and hash the parts, skip its leading zeros, and
/// count tallies down as they drop, then key collections by bound types. A first byte of
/// 1 slices out of order, which panics.
pub fn run(input: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	let bytes = Blob::copy_from_slice(input);
	let mid = bytes.len() / 2;
	let head = bytes.slice(..mid);
	let tail = bytes.slice(mid..);
	let inner = bytes.slice(1.min(mid)..mid);
	out.push(head.len() as u8);
	out.push(inner.len() as u8);
	out.extend_from_slice(&tail);
	out.push((head == tail) as u8);
	out.push((head.clone() == head) as u8);
	out.push((Blob::from(input.to_vec()) == bytes) as u8);
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
	keys(input, &bytes, &mut out);
	out
}

/// Collections keyed by `Blob`s, equal by their bytes wherever they are sliced from, and by
/// `Nibble`s, equal by their low bits: what each finds, keeps, and orders.
fn keys(input: &[u8], bytes: &Blob, out: &mut Vec<u8>) {
	let mut seen = HashSet::new();
	let mut counts = HashMap::new();
	let mut sorted = BTreeSet::new();
	let mut firsts = BTreeMap::new();
	for at in 0..bytes.len() {
		let part = bytes.slice(at..(at + 2).min(bytes.len()));
		out.push(seen.insert(part.clone()) as u8);
		*counts.entry(part.clone()).or_insert(0u8) += 1;
		sorted.insert(part.clone());
		firsts.entry(part).or_insert(at as u8);
	}
	out.push(seen.len() as u8);
	out.push(counts.len() as u8);
	// A copy of the bytes, which no part shares, finds the part equal to it.
	let probe = Blob::copy_from_slice(input.get(..2).unwrap_or(input));
	out.push(seen.contains(&probe) as u8);
	out.push(counts.get(&probe).copied().unwrap_or(0));
	out.push(sorted.contains(&probe) as u8);
	out.push(firsts.get(&probe).copied().unwrap_or(0xff));
	out.push(sorted.range(probe.clone()..).count() as u8);
	for (part, first) in &firsts {
		out.push(part.len() as u8);
		out.extend_from_slice(part);
		out.push(*first);
	}
	out.push(seen.remove(&probe) as u8);
	out.push(sorted.remove(&probe) as u8);
	out.push(seen.len() as u8);

	let mut nibbles = HashSet::new();
	let mut lasts = HashMap::new();
	let mut ordered = BTreeSet::new();
	let mut values = BTreeMap::new();
	for &byte in input {
		let key = Nibble::new(byte);
		out.push(nibbles.insert(key) as u8);
		out.push(lasts.insert(key, byte).unwrap_or(0xff));
		ordered.insert(key);
		values.insert(key, byte);
	}
	out.push(nibbles.len() as u8);
	for key in 0..16 {
		let key = Nibble::new(key * 17);
		out.push(nibbles.get(&key).map_or(0xff, |n| n.get()));
		out.push(lasts.get(&key).copied().unwrap_or(0xff));
	}
	for (key, value) in &values {
		out.push(key.get());
		out.push(*value);
	}
	out.extend(ordered.iter().map(|n| n.get()));
}

/// A `Held` of `None`, which `wrong.toml`'s binding would make `undefined`, as the outer
/// `None` is.
pub fn held() -> bool {
	Some(crate::raw::Held::new(None::<u8>)).is_some()
}

/// What `refused.toml`'s bindings can't stand in for, which `run` never reaches.
pub fn refused(count: &Arc<u32>) -> u8 {
	let mut bytes = Blob::new();
	replace(&mut bytes, Blob::from_static(b"replaced"));
	let held = crate::raw::Held::new(Tally::new(count));
	drop(held);
	let values = crate::raw::value::<1>() + crate::raw::value::<2>();
	crate::raw::zero::<u8>() + bytes.len() as u8 + values as u8
}

fn replace(target: &mut Blob, value: Blob) {
	*target = value;
}
