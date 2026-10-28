//! Functions that hand back byte slices and scalars, read apart by their callers or
//! kept whole, and called where that can't be seen: as values, again before they
//! return, and from their own drops.

use std::ops::ControlFlow;

/// The first byte and the rest.
fn pop(data: &[u8]) -> Option<(u8, &[u8])> {
	let (&b, rest) = data.split_first()?;
	Some((b, rest))
}

/// The front `n` bytes and the rest, or how many it needed.
fn front(data: &[u8], n: usize) -> Result<(&[u8], &[u8]), u8> {
	data.split_at_checked(n).ok_or(n as u8)
}

/// Halves, with no variant to return.
fn halves(data: &[u8]) -> (&[u8], &[u8]) {
	data.split_at(data.len() / 2)
}

/// `n` bytes skipped one at a time, by calls of itself, with how deep it went.
fn skip(data: &[u8], n: usize) -> Result<(&[u8], usize), u8> {
	if n == 0 {
		return Ok((data, 0));
	}
	let (_, rest) = data.split_first().ok_or(n as u8)?;
	let (rest, depth) = skip(rest, n - 1)?;
	Ok((rest, depth + 1))
}

/// What it was given, an object of the result's shape.
fn pass(r: Result<(u8, &[u8]), Option<u8>>) -> Result<(u8, &[u8]), Option<u8>> {
	r
}

/// The first byte if it is odd, breaking with it, or else the rest.
fn odd(data: &[u8]) -> ControlFlow<u8, (u8, &[u8])> {
	match data.split_first() {
		Some((&b, _)) if b % 2 == 1 => ControlFlow::Break(b),
		Some((&b, rest)) => ControlFlow::Continue((b, rest)),
		None => ControlFlow::Break(0),
	}
}

/// A drop that calls `pop` again, after its owner's result is made.
struct Again<'a>(&'a mut Vec<u8>);

impl Drop for Again<'_> {
	fn drop(&mut self) {
		if let Some((b, rest)) = pop(&[0x77, 1, 2]) {
			self.0.push(b ^ rest.len() as u8);
		}
	}
}

/// `pop` of `data`, with a drop between making its result and returning it that
/// calls `pop` too.
fn guarded<'a>(data: &'a [u8], out: &mut Vec<u8>) -> Option<(u8, &'a [u8])> {
	let _again = Again(out);
	pop(data)
}

/// A slice kept whole in a field.
struct Held<'a> {
	rest: &'a [u8],
}

pub fn returns(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 8 {
		0 => {
			let mut rest = data;
			while let Some((b, tail)) = pop(rest) {
				out.push(b);
				rest = tail;
			}
			match front(data, usize::from(kind / 8)) {
				Ok((a, b)) => {
					out.extend_from_slice(b);
					out.extend_from_slice(a);
				}
				Err(n) => out.push(n),
			}
		}
		1 => {
			// As values: a function pointer, and a function std's combinator calls.
			let f: fn(&[u8]) -> Option<(u8, &[u8])> = pop;
			if let Some((b, rest)) = f(data) {
				out.push(b);
				out.push(rest.len() as u8);
			}
			let popped = Some(data).and_then(pop);
			out.push(popped.map_or(0xff, |(b, rest)| b ^ rest.len() as u8));
		}
		2 => {
			let (a, b) = halves(data);
			let (c, d) = halves(b);
			out.extend_from_slice(d);
			out.extend_from_slice(c);
			out.extend_from_slice(a);
		}
		3 => match skip(data, usize::from(kind / 8)) {
			Ok((rest, depth)) => {
				out.push(depth as u8);
				out.extend_from_slice(rest);
			}
			Err(n) => out.push(0xf0 ^ n),
		},
		4 => {
			let made = match data.split_first() {
				Some((&b, rest)) => Ok((b, rest)),
				None => Err(data.len().checked_sub(1).map(|n| n as u8)),
			};
			match pass(made) {
				Ok((b, rest)) => {
					out.push(b);
					out.extend_from_slice(rest);
				}
				Err(Some(n)) => out.push(n),
				Err(None) => out.push(0xee),
			}
			match pass(Err(data.first().copied())) {
				Ok(_) => out.push(0),
				Err(e) => out.push(e.unwrap_or(0xdd)),
			}
		}
		5 => match odd(data) {
			ControlFlow::Continue((b, rest)) => {
				out.push(b);
				out.extend_from_slice(rest);
			}
			ControlFlow::Break(b) => out.push(0x80 | b),
		},
		6 => {
			let popped = guarded(data, out);
			match popped {
				Some((b, rest)) => {
					out.push(b);
					out.extend_from_slice(rest);
				}
				None => out.push(0xcc),
			}
		}
		_ => {
			// Kept whole, by a field, and by std's combinators over closures of one shape.
			let held = Held {
				rest: front(data, 1).map_or(data, |(_, b)| b),
			};
			out.extend_from_slice(held.rest);
			let a = data.split_first().map(|(&b, rest)| (b, rest));
			let b = data.split_at_checked(1).map(|(b, rest)| (b[0] ^ 0x5a, rest));
			for (b, rest) in a.into_iter().chain(b) {
				out.push(b ^ rest.len() as u8);
			}
		}
	}
}
