//! Mutexes on one thread: made by `Default`, read and written through a guard, unlocked
//! as it drops, read by `get_mut` without locking, replaced through a reference, and
//! locked again while a guard is held, which panics as wasm32's does.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

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
	let mut held: Option<MutexGuard<'_, Tally>> = None;
	for &op in data {
		match op % 7 {
			0 => {
				let mut tally = shared.lock().unwrap_or_else(PoisonError::into_inner);
				tally.n += u64::from(op);
				tally.seen.push(op);
			}
			1 => {
				let tally = other.lock().unwrap();
				out.push(tally.n as u8);
				out.push(tally.seen.len() as u8);
			}
			2 => own.get_mut().unwrap().push(op),
			3 => {
				*number.lock().unwrap() += u32::from(op);
				out.push(*number.lock().unwrap() as u8);
			}
			// Locking while a guard from before is held panics.
			4 => held = Some(shared.lock().unwrap()),
			5 => held = None,
			// Both locked before, so each has handed out a guard of its own.
			_ => {
				let fresh = Mutex::new(vec![op]);
				drop(fresh.lock());
				drop(own.lock());
				replace(&mut own, fresh);
				own.lock().unwrap().push(op);
				out.push(own.lock().unwrap().len() as u8);
			}
		}
	}
	out.push(held.is_some() as u8);
	drop(held);
	out.extend_from_slice(own.get_mut().unwrap_or_else(PoisonError::into_inner));
	out.push(Arc::strong_count(&shared) as u8);
}

/// Replaces the mutex `dst` borrows, as an assignment through a reference does.
fn replace(dst: &mut Mutex<Vec<u8>>, src: Mutex<Vec<u8>>) {
	*dst = src;
}
