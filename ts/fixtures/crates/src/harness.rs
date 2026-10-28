//! The entry point both sides of the differential test run.

use std::sync::{Arc, Mutex};

use crates_dep::{Countdown, Guard, Point, Weigh};

/// The package's own type, which the dependency's generic function ranks.
struct Item(u8);

impl Weigh for Item {
	fn weight(&self) -> u32 {
		self.0 as u32 % 7
	}
}

pub fn run(input: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	// Only the dependency writes into a `Point`, so the copy must not share it.
	if let [a, b, ..] = input {
		let p = Point {
			x: *a as i32,
			y: *b as i32,
		};
		let mut q = p;
		q.shift(1, -1);
		out.extend([p.x as u8, p.y as u8, q.x as u8, q.y as u8]);
	}
	let items: Vec<Item> = input.iter().map(|&byte| Item(byte)).collect();
	if let Some(Item(byte)) = crates_dep::heaviest(&items) {
		out.push(*byte);
	}
	// The dependency's drops, where native Rust runs them.
	let log = Arc::new(Mutex::new(Vec::new()));
	{
		let guards: Vec<Guard> = input.iter().take(3).map(|&id| Guard::new(id, &log)).collect();
		let held = Guard::new(0xff, &log);
		drop(guards);
		let _late = Guard::new(0xfe, &log);
		drop(held);
	}
	out.extend(log.lock().unwrap().iter());
	for n in Countdown::new(input.len().min(4) as u8) {
		out.push(n);
	}
	out.push(crates_dep::first(input));
	out.push(crates_dep::last(input));
	out
}
