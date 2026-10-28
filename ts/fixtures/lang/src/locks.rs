//! Mutexes on one thread, where a panic aborts and so never poisons one: made by
//! `Default`, read and written through a guard, recovered from a `PoisonError` that never
//! comes, read by `get_mut` without locking, tried while held and free, and locked again
//! while held, which panics as wasm32's does.

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError, TryLockError};

/// A value a mutex holds, which `Default` makes.
#[derive(Default)]
struct Tally {
	n: u64,
	seen: Vec<u8>,
}

/// A mutex that debugs through `try_lock`, as kio's `Lock` does.
struct Lock(Mutex<u32>);

impl fmt::Debug for Lock {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.0.try_lock() {
			Ok(guard) => f.debug_tuple("Lock").field(&*guard).finish(),
			Err(_) => f.debug_tuple("Lock").field(&"<locked>").finish(),
		}
	}
}

/// Each byte an operation on the mutexes, its value the operand; each puts what it read.
pub fn locks(data: &[u8], out: &mut Vec<u8>) {
	let shared: Arc<Mutex<Tally>> = Arc::default();
	let other = shared.clone();
	let mut own: Mutex<Vec<u8>> = Mutex::default();
	let number: Mutex<u32> = Mutex::default();
	let lock = Lock(Mutex::new(7));
	for &op in data {
		match op % 8 {
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
			// Tried while another guard holds it, and again once that guard drops.
			4 => {
				let held = shared.lock().unwrap();
				out.push(matches!(other.try_lock(), Err(TryLockError::WouldBlock)) as u8);
				drop(held);
				match other.try_lock() {
					Ok(tally) => out.push(tally.seen.len() as u8),
					Err(TryLockError::WouldBlock) => out.push(0xfe),
					Err(TryLockError::Poisoned(err)) => out.push(err.into_inner().n as u8),
				}
			}
			// Debugged free and held, and a guard kept in a binding until the block ends.
			5 => {
				*lock.0.lock().unwrap() += u32::from(op);
				let free = format!("{lock:?}");
				let guard = lock.0.try_lock();
				let held = format!("{lock:?}");
				out.extend_from_slice(format!("{free} {held} {}", guard.is_ok()).as_bytes());
			}
			6 => out.push(std::thread::panicking() as u8),
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

/// Locks a mutex while a guard of it is held, if the first byte is odd, which panics.
pub fn relock(data: &[u8], out: &mut Vec<u8>) {
	let mutex = Mutex::new(data.len() as u8);
	let guard = mutex.lock().unwrap();
	out.push(*guard);
	if data.first().is_some_and(|b| b % 2 == 1) {
		out.push(*mutex.lock().unwrap());
	}
	drop(guard);
	out.push(*mutex.lock().unwrap());
}
