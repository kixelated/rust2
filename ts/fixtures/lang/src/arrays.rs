//! Arrays `array::from_fn` makes, calling its function on each index in order: of bytes,
//! wide integers, atomics each its own, strings, nested arrays, none at all, and a length
//! a const parameter gives.

use std::sync::atomic::{AtomicU64, Ordering};

/// Each byte an operation, its value the operand; each puts what it read.
pub fn from_fns(data: &[u8], out: &mut Vec<u8>) {
	for &op in data {
		let s = match op % 5 {
			// The order the function sees, through what it captured by `&mut`.
			0 => {
				let mut seen = Vec::new();
				let bytes: [u8; 4] = std::array::from_fn(|i| {
					seen.push(i);
					op.wrapping_mul(i as u8 + 1)
				});
				let wide: [u64; 3] = std::array::from_fn(|i| u64::from(op) << (i * 30));
				format!("{bytes:?} {wide:?} {seen:?}")
			}
			// kio's: each atomic its own, which a store to one doesn't change in another.
			1 => {
				let counts: [AtomicU64; 8] = std::array::from_fn(|_| AtomicU64::new(u64::from(op)));
				counts[usize::from(op % 8)].fetch_add(1, Ordering::Relaxed);
				let copy: [AtomicU64; 8] = std::array::from_fn(|i| AtomicU64::new(counts[i].load(Ordering::Relaxed)));
				copy[0].store(0, Ordering::Relaxed);
				let loads: [u64; 8] = std::array::from_fn(|i| counts[i].load(Ordering::Relaxed));
				format!("{loads:?} {}", copy[0].load(Ordering::Relaxed))
			}
			// A function that owns what it captured, which drops after the last call.
			2 => {
				let word = format!("w{op}");
				let words: [String; 3] = std::array::from_fn(move |i| format!("{word}{i}"));
				let none: [String; 0] = std::array::from_fn(|i| i.to_string());
				format!("{words:?} {none:?}")
			}
			3 => {
				let grid: [[u8; 2]; 3] =
					std::array::from_fn(|i| std::array::from_fn(|j| op.wrapping_add((i * 2 + j) as u8)));
				let named: [u16; 4] = std::array::from_fn(doubled);
				format!("{grid:?} {named:?}")
			}
			_ => format!("{:?} {:?}", counted::<2>(op), counted::<5>(op)),
		};
		out.extend_from_slice(s.as_bytes());
		out.push(b'\n');
	}
}

fn doubled(i: usize) -> u16 {
	i as u16 * 2
}

/// An array as long as `N` says, of `N` instances.
fn counted<const N: usize>(op: u8) -> [usize; N] {
	std::array::from_fn(|i| i * N + usize::from(op))
}
