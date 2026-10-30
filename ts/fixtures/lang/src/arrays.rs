//! Arrays `array::from_fn` makes, calling its function on each index in order: of bytes,
//! wide integers, atomics each its own, strings, nested arrays, none at all, and a length
//! a const parameter gives; and arrays as slices, written through.

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

/// Arrays as slices, of bytes and of other values, empty ones included; each byte an
/// operation, its value the operand. Writes through `as_mut_slice` land in the array.
pub fn as_slices(data: &[u8], out: &mut Vec<u8>) {
	let mut bytes = [0u8; 4];
	let mut wide = [0u16; 3];
	let mut words = [String::new(), String::new()];
	let mut grid = [[0u8; 2]; 3];
	let mut pairs = [[0u32; 2]; 2];
	let mut none: [u16; 0] = [];
	let mut empty: [u8; 0] = [];
	for &op in data {
		let i = usize::from(op);
		match op % 8 {
			0 => {
				let s = bytes.as_mut_slice();
				s[i % 4] = op;
				s.rotate_left(1);
			}
			1 => {
				wide.as_mut_slice()[i % 3] = u16::from(op) << 7;
				wide.as_mut_slice().rotate_right(1);
			}
			2 => words.as_mut_slice()[i % 2].push(char::from(b'a' + op % 26)),
			// An element of one array as a slice, as the slice of the outer array lends it.
			3 => {
				let row = &mut grid.as_mut_slice()[i % 3];
				row.as_mut_slice()[i % 2] = op;
				let read = row.as_slice()[(i + 1) % 2];
				grid[(i + 1) % 3][0] ^= read;
			}
			4 => {
				let all = pairs.as_mut_slice();
				all[i % 2]
					.as_mut_slice()
					.copy_from_slice(&[u32::from(op), u32::from(op) << 20]);
				all.swap(0, 1);
			}
			5 => {
				none.as_mut_slice().reverse();
				empty.as_mut_slice().fill(op);
				out.push(none.as_slice().len() as u8 + empty.as_slice().len() as u8);
			}
			6 => written(&mut bytes, &mut wide, op),
			_ => {
				let sum: u32 = wide.as_slice().iter().map(|&w| u32::from(w)).sum();
				out.extend_from_slice(&sum.to_le_bytes());
				out.extend_from_slice(bytes.as_slice());
			}
		}
	}
	let s = format!(
		"{:?} {:?} {:?} {:?} {:?} {:?} {:?}",
		bytes.as_slice(),
		wide.as_slice(),
		words.as_slice(),
		grid.as_slice(),
		pairs.as_slice(),
		none.as_slice(),
		empty.as_slice()
	);
	out.extend_from_slice(s.as_bytes());
}

/// Arrays lent by `&mut`, written through their slices.
fn written(bytes: &mut [u8; 4], wide: &mut [u16; 3], op: u8) {
	let s = bytes.as_mut_slice();
	s[3] = s[0].wrapping_add(op);
	let w = wide.as_mut_slice();
	w[0] = u16::from(op);
	w.swap(1, 2);
	w[1..].fill(u16::from(op) + 1);
}
