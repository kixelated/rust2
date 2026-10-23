//! Constructs outside the subset: each root fails translation with a span, and no
//! output is written.

#![allow(dead_code, reason = "each item is a translation root, not called from Rust")]

pub fn float(a: f64) -> f64 {
	a * 2.0
}

pub fn character(c: char) -> u32 {
	c as u32
}

pub fn dynamic(f: &dyn Fn(u8) -> u8) -> u8 {
	f(1)
}

/// JS has no reference to a number: the caller's `u32` would not change.
pub fn bump(x: &mut u32) {
	*x += 1;
}

/// Each element would be a copy, so the increments would be lost.
pub fn increment(v: &mut [u32]) {
	for x in v {
		*x += 1;
	}
}

/// `None` and `Some(None)` would both be `undefined`.
pub fn nested(x: Option<Option<u8>>) -> bool {
	x.is_some()
}

pub fn sorted(v: &mut [u8]) {
	v.sort();
}

pub fn iterators(v: &[u8]) -> usize {
	v.iter().filter(|b| **b > 1).count()
}

pub struct Guard;

impl Drop for Guard {
	fn drop(&mut self) {}
}

pub fn guard() -> Guard {
	Guard
}

pub async fn later() -> u8 {
	1
}

pub fn formatted(n: u8) {
	if n > 3 {
		panic!("too many: {n}");
	}
}

pub fn captured() -> u8 {
	let mut n = 1;
	let f = move || n + 1;
	n += 10;
	f() + n
}

pub fn or_bindings(x: Result<u8, u8>) -> u8 {
	match x {
		Ok(v) | Err(v) => v,
	}
}

static COUNT: u8 = 0;

pub fn statics() -> u8 {
	COUNT
}

pub fn generic<T: Copy>(t: T) -> T {
	t
}
