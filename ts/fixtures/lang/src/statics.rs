//! `static` atomics, each one value every use shares: of every kind, at module level and
//! in a function, as kio's `waiter` keeps its ids, starting from a value of the type's
//! extremes. Each case leaves every static as it found it, so a fresh instance and a
//! reused one agree.

use std::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, SeqCst};
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicI64, AtomicU64, AtomicUsize};

static COUNT: AtomicU64 = AtomicU64::new(u64::MAX - 2);
static SIGNED: AtomicI8 = AtomicI8::new(-128);
static FLAG: AtomicBool = AtomicBool::new(true);
static WIDE: AtomicI64 = AtomicI64::new(i64::MIN + 1);
static SIZE: AtomicUsize = AtomicUsize::new(7);
/// Read, never written: the static's memory.
static TABLE: [u8; 4] = [9, 8, 7, 6];

/// Each byte an operation on the statics; each puts what it read. First, what each holds,
/// which only the value it starts with can be.
pub fn statics(data: &[u8], out: &mut Vec<u8>) {
	out.extend(COUNT.load(Relaxed).to_le_bytes());
	out.extend(WIDE.load(Relaxed).to_le_bytes());
	out.extend([
		SIGNED.load(Relaxed) as u8,
		u8::from(FLAG.load(Relaxed)),
		SIZE.load(Relaxed) as u8,
	]);
	out.push(next(false));
	for &op in data {
		let n = op / 6;
		match op % 6 {
			// Wraps past the type's end, as an atomic's arithmetic does.
			0 => out.extend(COUNT.fetch_add(u64::from(n), AcqRel).to_le_bytes()),
			1 => out.push(SIGNED.fetch_sub(n as i8, SeqCst) as u8),
			2 => out.push(u8::from(FLAG.fetch_xor(n % 2 == 0, Relaxed))),
			3 => out.extend(WIDE.swap(-i64::from(n), Acquire).to_le_bytes()),
			4 => {
				let seen = SIZE.load(Relaxed);
				out.push(
					match SIZE.compare_exchange(seen, seen.wrapping_mul(usize::from(n)), SeqCst, Relaxed) {
						Ok(old) => old as u8,
						Err(_) => 0xfd,
					},
				);
			}
			5 if n % 2 == 0 => out.push(next(false)),
			// One table, wherever it is borrowed.
			_ => out.extend([table()[usize::from(n) % 4], u8::from(std::ptr::eq(table(), table()))]),
		}
	}
	COUNT.store(u64::MAX - 2, Relaxed);
	SIGNED.store(-128, Relaxed);
	FLAG.store(true, Relaxed);
	WIDE.store(i64::MIN + 1, Relaxed);
	SIZE.store(7, Relaxed);
	next(true);
}

fn table() -> &'static [u8; 4] {
	&TABLE
}

/// The next id from a static in a function, as kio's `waiter` takes one; starts over if `reset`.
fn next(reset: bool) -> u8 {
	static NEXT: AtomicU64 = AtomicU64::new(1);
	match reset {
		true => {
			NEXT.store(1, Relaxed);
			0
		}
		false => NEXT.fetch_add(1, Relaxed) as u8,
	}
}
