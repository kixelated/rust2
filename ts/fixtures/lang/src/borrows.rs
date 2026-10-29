//! `Cell`s and `RefCell`s on one thread: read, written, replaced, and taken, of numbers,
//! tuples, and values that drop; borrowed shared and exclusive, held in bindings and
//! dropped; and borrowed again while held, which panics as wasm32's does.

use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};

/// Logs its id when it drops.
struct Noisy {
	id: u8,
	log: Arc<Mutex<Vec<u8>>>,
}

impl Drop for Noisy {
	fn drop(&mut self) {
		self.log.lock().unwrap().push(self.id);
	}
}

/// What a shared reference changes, as kio's `Polling` holds it.
#[derive(Default)]
struct Polling {
	current: Cell<Option<(u64, u64)>>,
	held: RefCell<Vec<u8>>,
}

/// Each byte an operation on the cells, its value the operand; each puts what it read and
/// what dropped.
pub fn borrows(data: &[u8], out: &mut Vec<u8>) {
	let log = Arc::new(Mutex::new(Vec::new()));
	let count = Cell::new(0u32);
	let pair = Cell::new((0u64, 0u64));
	let noisy: Cell<Option<Noisy>> = Cell::new(None);
	let polling = Polling::default();
	let shared: RefCell<Vec<u8>> = RefCell::new(Vec::new());
	let owned = RefCell::new(Noisy {
		id: 0xaa,
		log: log.clone(),
	});
	for &op in data {
		match op % 10 {
			0 => count.set(count.get().wrapping_add(u32::from(op))),
			// A copy out, written, leaves the cell as it was.
			1 => {
				let mut copy = pair.get();
				copy.0 += u64::from(op);
				out.push(pair.get().0 as u8);
				pair.set((copy.1, copy.0));
				out.push(pair.get().1 as u8);
			}
			2 => out.push(count.replace(u32::from(op)) as u8),
			// Drops the value it held.
			3 => noisy.set(Some(Noisy {
				id: op,
				log: log.clone(),
			})),
			4 => {
				let taken = noisy.take();
				out.push(taken.as_ref().map_or(0xfd, |n| n.id));
			}
			5 => {
				let outer = polling.current.replace(Some((u64::from(op), 1)));
				polling.held.borrow_mut().push(op);
				out.push(outer.map_or(0xfd, |(a, _)| a as u8));
				out.push(polling.held.borrow().len() as u8);
			}
			// Two shared borrows at once.
			6 => {
				let a = shared.borrow();
				let b = shared.borrow();
				out.push((a.len() + b.len()) as u8);
			}
			7 => {
				let mut w = shared.borrow_mut();
				w.push(op);
				out.push(w.len() as u8);
				drop(w);
				out.push(shared.borrow().len() as u8);
			}
			// Replaced through a borrow, which drops the old value.
			8 => {
				*owned.borrow_mut() = Noisy {
					id: op,
					log: log.clone(),
				};
				out.push(owned.borrow().id);
			}
			// Borrowed while an exclusive borrow is held, or exclusively while any is.
			_ => match op >> 4 {
				0xd => {
					let _r = shared.borrow();
					let _w = shared.borrow_mut();
				}
				0xe => {
					let _w = shared.borrow_mut();
					let _r = shared.borrow();
				}
				0xf => {
					let _w = shared.borrow_mut();
					let _again = shared.borrow_mut();
				}
				_ => {
					drop(shared.borrow_mut());
					out.push(shared.borrow().len() as u8);
				}
			},
		}
		flush(&log, out);
	}
	out.push(count.get() as u8);
	out.push(polling.current.get().map_or(0xfd, |(a, b)| (a + b) as u8));
	drop(noisy);
	drop(owned);
	flush(&log, out);
}

/// Puts what dropped since the last flush.
fn flush(log: &Mutex<Vec<u8>>, out: &mut Vec<u8>) {
	let mut log = log.lock().unwrap();
	out.extend_from_slice(&log);
	log.clear();
}
