//! Mutexes on one thread, where a panic aborts and so never poisons one: made by
//! `Default`, read and written through a guard, recovered from a `PoisonError` that never
//! comes, and read by `get_mut` without locking.

use std::sync::{Arc, Mutex, PoisonError};

/// A value a mutex holds, which `Default` makes.
#[derive(Default)]
struct Tally {
	n: u64,
	seen: Vec<u8>,
}

/// Each byte an operation on the mutexes, its value the operand; each puts what it read.
pub fn locks(data: &[u8], out: &mut Vec<u8>) {
	let shared: Arc<Mutex<Tally>> = Arc::default();
	let other = shared.clone();
	let mut own: Mutex<Vec<u8>> = Mutex::default();
	let number: Mutex<u32> = Mutex::default();
	for &op in data {
		match op % 5 {
			0 => {
				let mut tally = shared.lock().unwrap_or_else(PoisonError::into_inner);
				tally.n += u64::from(op);
				tally.seen.push(op);
			}
			1 => {
				let tally = other.lock().expect("never poisoned");
				out.push(tally.n as u8);
				out.push(tally.seen.len() as u8);
			}
			2 => own.get_mut().unwrap().push(op),
			3 => {
				*number.lock().unwrap() += u32::from(op);
				out.push(*number.lock().unwrap() as u8);
			}
			// Matched, as a caller that tells the two apart does.
			_ => match shared.lock() {
				Ok(tally) => out.push(tally.seen.len() as u8),
				Err(err) => out.push(err.into_inner().n as u8 ^ 0xff),
			},
		}
	}
	out.extend_from_slice(own.get_mut().unwrap_or_else(PoisonError::into_inner));
	out.push(Arc::strong_count(&shared) as u8);
}
