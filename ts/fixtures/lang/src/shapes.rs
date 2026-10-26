//! Control flow, patterns, aggregates, closures, and traits.

use std::convert::Infallible;
use std::ops::ControlFlow;
use std::sync::Mutex;
use std::task::Poll;

use crate::fmt::Pair;

/// A `Copy` struct, which Rust copies where JS would share.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
	pub x: i32,
	pub y: i32,
}

impl Point {
	pub fn new(x: i32, y: i32) -> Self {
		Self { x, y }
	}

	/// Changes `self`, which a caller's copy must not see.
	pub fn shift(mut self, by: i32) -> Self {
		self.x += by;
		self
	}

	pub fn bump(&mut self) {
		self.y += 1;
	}
}

/// A tuple struct.
#[derive(Clone, Copy)]
pub struct Meters(pub u64);

/// A tuple struct that is not `Copy`, so struct update syntax reads its base in place.
#[derive(Debug)]
pub struct Moved(pub u8, pub u16);

/// An enum with data, and one without.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
	Dot,
	Line(Point, Point),
	Rect { corner: Point, width: u16, height: u16 },
}

/// Discriminants a cast truncates, or reads with another sign.
#[derive(Clone, Copy)]
#[repr(u16)]
pub enum Wide {
	Low = 1,
	High = 300,
	Top = u16::MAX,
}

#[derive(Clone, Copy)]
#[repr(i8)]
pub enum Signed {
	Neg = -1,
	Min = i8::MIN,
	Pos = 100,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
	North = 1,
	East = 2,
	South = 4,
	West = 8,
}

pub trait Area {
	const SIDES: u8;

	fn area(&self) -> u64;

	/// A default method, monomorphized for each type.
	fn describe(&self, out: &mut Vec<u8>) {
		out.push(Self::SIDES);
		out.extend_from_slice(&self.area().to_be_bytes());
	}
}

impl Area for Shape {
	const SIDES: u8 = 4;

	fn area(&self) -> u64 {
		match *self {
			Shape::Dot => 0,
			Shape::Line(..) => 0,
			Shape::Rect { width, height, .. } => width as u64 * height as u64,
		}
	}
}

impl Area for Meters {
	const SIDES: u8 = 0;

	fn area(&self) -> u64 {
		self.0
	}

	fn describe(&self, out: &mut Vec<u8>) {
		out.push(0xaa);
		out.extend_from_slice(&self.0.to_be_bytes());
	}
}

fn total<T: Area>(items: &[T], out: &mut Vec<u8>) -> u64 {
	let mut sum = 0u64;
	for item in items {
		item.describe(out);
		sum = sum.saturating_add(item.area());
	}
	sum
}

impl From<Dir> for u8 {
	fn from(dir: Dir) -> u8 {
		dir as u8
	}
}

fn dir(byte: u8) -> Option<Dir> {
	match byte % 5 {
		0 => Some(Dir::North),
		1 => Some(Dir::East),
		2 => Some(Dir::South),
		3 => Some(Dir::West),
		_ => None,
	}
}

fn shape(data: &[u8]) -> Shape {
	let Some((&kind, rest)) = data.split_first() else {
		return Shape::Dot;
	};
	let at = |i: usize| rest.get(i).copied().unwrap_or(0) as i32 - 128;
	match kind % 3 {
		0 => Shape::Dot,
		1 => Shape::Line(Point::new(at(0), at(1)), Point::new(at(2), at(3))),
		_ => Shape::Rect {
			corner: Point::new(at(0), at(1)),
			width: rest.get(2).copied().unwrap_or(0) as u16 * 300,
			height: rest.get(3).copied().unwrap_or(0) as u16,
		},
	}
}

/// `Copy` aggregates: a copy changes apart from its original.
pub fn copies(data: &[u8], out: &mut Vec<u8>) {
	let p = Point::new(data.first().copied().unwrap_or(0) as i32, 7);
	let mut q = p;
	q.x += 100;
	q.bump();
	let r = p.shift(5);
	let mut pair = (p, 3u8);
	let saved = pair;
	pair.0.y = -1;
	pair.1 += 1;
	let mut grid = [p; 3];
	grid[1].x = 9;
	let row = grid;
	grid[2] = q;
	for v in [
		p.x, p.y, q.x, q.y, r.x, saved.0.y, pair.0.y, row[1].x, row[2].x, grid[2].x,
	] {
		out.extend_from_slice(&v.to_be_bytes());
	}
	out.push(saved.1);
	out.push(pair.1);
	out.push((p == saved.0) as u8);

	let mut a = Point::new(1, 2);
	let mut b = Point::new(3, 4);
	std::mem::swap(&mut a, &mut b);
	let old = std::mem::replace(&mut a, Point::new(5, 6));
	out.extend_from_slice(&[a.x as u8, b.x as u8, old.x as u8]);

	// A box is its contents: a write through it, or through a borrow of it, changes them.
	let mut boxed = Box::new(Moved(data.len() as u8, 0));
	boxed.1 += 1;
	let held = &mut boxed;
	held.1 += 2;
	out.extend_from_slice(&[boxed.0]);
	out.extend_from_slice(&boxed.1.to_be_bytes());

	// A tuple keeps the values it was made of, though the variable they came from changes.
	let mut n = data.len() as u32;
	let both = (n, n);
	n = 9;
	out.extend_from_slice(&(both.0 + both.1 + n).to_be_bytes());

	// A value computed from a call keeps that call before a later one, though it is read
	// after.
	// A conversion reads its bytes before they change.
	let mut word = [0, 0, 0, data.len() as u8];
	let converted = u32::from_be_bytes(word);
	word[0] = 9;
	out.extend_from_slice(&(converted + word[0] as u32).to_be_bytes());

	// An argument copied keeps its value, though the callee changes its source through
	// another.
	let mut cell = [data.len() as u8];
	out.push(write_then_read(cell, &mut cell));

	// A result returning an argument is its own copy, though the source changes after.
	let mut source = [data.len() as u8];
	let returned = same(source);
	source[0] = 9;
	out.extend_from_slice(&[returned[0], source[0]]);
	out.push(copy_then_write((data.len() as u8, 0)));

	let marked = mark(out, 1) == 0;
	mark(out, 2);
	out.push(marked as u8);

	let mut opt = Some(Meters(9));
	let taken = opt.take();
	out.push(opt.is_none() as u8);
	out.extend_from_slice(&taken.map_or(0, |m| m.0).to_be_bytes());

	// A `Copy` payload leaves a `Result` as a copy.
	let res: Result<Point, u8> = Ok(p);
	let mut first = res.unwrap();
	first.x = 2;
	let fail: Result<u8, Point> = Err(q);
	let mut err = fail.unwrap_err();
	err.y = 3;
	out.extend_from_slice(&[
		first.x as u8,
		res.unwrap().x as u8,
		err.y as u8,
		fail.unwrap_err().y as u8,
	]);
}

/// `a`'s byte, after writing `b`'s.
#[inline(never)]
fn write_then_read(a: [u8; 1], b: &mut [u8; 1]) -> u8 {
	b[0] = 9;
	a[0]
}

/// A copy of `a` read after `a` changes in place, which it doesn't see.
fn copy_then_write(mut a: (u8, u8)) -> u8 {
	let b = a;
	a.0 = 9;
	b.0.wrapping_add(a.0)
}

/// `a` itself.
#[inline(never)]
fn same(a: [u8; 1]) -> [u8; 1] {
	a
}

/// Logs `b`, returning how much is logged.
#[inline(never)]
fn mark(out: &mut Vec<u8>, b: u8) -> usize {
	out.push(b);
	out.len()
}

/// Aggregates written with their fields out of declaration order evaluate them as
/// written: each field logs as it runs.
pub fn order(data: &[u8], out: &mut Vec<u8>) {
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	let rect = Shape::Rect {
		height: mark(out, at(0)) as u16,
		width: mark(out, at(1)) as u16,
		corner: Point {
			y: mark(out, 2) as i32,
			x: mark(out, 3) as i32,
		},
	};
	let line = Shape::Line {
		1: Point::new(mark(out, 4) as i32, 0),
		0: Point::new(mark(out, 5) as i32, 0),
	};
	let pair = Pair {
		1: mark(out, 6) as i64,
		0: mark(out, 7) as u8,
	};
	// A local read before a later field changes it.
	let mut n = [at(2)];
	let read = Point {
		y: n[0] as i32,
		x: increment(&mut n) as i32,
	};
	// The base is read after the fields, which change it.
	let mut base = Moved(at(4), 0);
	let updated = Moved {
		1: reset(&mut base),
		..base
	};
	// Only the branch taken runs its fields.
	let taken = match at(3) % 2 {
		0 => Shape::Rect {
			width: mark(out, 8) as u16,
			height: 1,
			corner: Point::new(0, mark(out, 9) as i32),
		},
		_ => Shape::Dot,
	};
	let range = std::ops::Range {
		end: mark(out, 10),
		start: mark(out, 11) / 2,
	};
	let mut sum = 0;
	for i in (std::ops::Range {
		end: mark(out, 12),
		start: mark(out, 13) - 4,
	}) {
		sum += i;
	}
	let len = data.len();
	// The sliced value comes before the range's fields.
	let slice = &logged(data, out)[std::ops::Range {
		end: mark(out, 14).min(len),
		start: mark(out, 15).min(len) / 2,
	}];
	let text = format!(
		"{rect:?} {line:?} {pair:?} {read:?} {updated:?} {taken:?} {} {} {sum} {slice:?}",
		range.start, range.end
	);
	out.extend_from_slice(text.as_bytes());
	indexed(data, out);
}

/// Indexing, calls, and assignments whose later operands need statements of their own
/// evaluate what comes before those first: each logs as it runs, or changes what an
/// earlier operand read.
fn indexed(data: &[u8], out: &mut Vec<u8>) {
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	// The vector, then the index.
	let first = listed(out, at(0))[{
		out.push(0xe1);
		1
	}];
	// The slice, the range, then `dest`.
	let mut bytes = [at(1), at(2), at(3), 4];
	logged_mut(&mut bytes, out).copy_within(
		{
			out.push(0xe3);
			1
		}..3,
		{
			out.push(0xe4);
			0
		},
	);
	logged_mut(&mut bytes, out).copy_within(2.., {
		out.push(0xe5);
		1
	});
	// Without an end, the slice is read once for its length too.
	logged_mut(&mut bytes, out).copy_within(3.., 0);
	// A slice, then its index.
	let second = logged(&bytes, out)[{
		out.push(0xe6);
		1
	}];
	// A function pointer is read before its argument, which changes it.
	let mut f: fn(u8) -> u8 = halve;
	let called = f({
		f = twice;
		at(4)
	});
	// A compound assignment's value is read before its place's index, which changes it.
	let mut n = at(5);
	let mut slots = [0u8; 2];
	slots[{
		n = n.wrapping_add(1);
		0
	}] += n;
	out.extend_from_slice(&[first, second, called, f(1), slots[0], n]);
	out.extend_from_slice(&bytes);
}

/// A vector of `b`, logging that it was made.
#[inline(never)]
fn listed(out: &mut Vec<u8>, b: u8) -> Vec<u8> {
	out.push(0xe0);
	[b, b ^ 1].to_vec()
}

/// `s`, logging that it was read.
#[inline(never)]
fn logged_mut<'a>(s: &'a mut [u8], out: &mut Vec<u8>) -> &'a mut [u8] {
	out.push(0xe2);
	s
}

#[inline(never)]
fn halve(b: u8) -> u8 {
	b / 2
}

#[inline(never)]
fn twice(b: u8) -> u8 {
	b.wrapping_add(b)
}

/// `data`, logging that it was read.
#[inline(never)]
fn logged<'a>(data: &'a [u8], out: &mut Vec<u8>) -> &'a [u8] {
	out.push(0xee);
	data
}

/// `n`'s byte, incremented.
#[inline(never)]
fn increment(n: &mut [u8; 1]) -> u8 {
	n[0] = n[0].wrapping_add(1);
	n[0]
}

/// Changes `m`'s first field.
#[inline(never)]
fn reset(m: &mut Moved) -> u16 {
	m.0 = 9;
	1
}

/// Fieldless enums cast to every width, past their discriminant's range and sign.
pub fn discriminants(data: &[u8], out: &mut Vec<u8>) {
	for &b in data {
		let wide = match b % 3 {
			0 => Wide::Low,
			1 => Wide::High,
			_ => Wide::Top,
		};
		let signed = match b % 3 {
			0 => Signed::Neg,
			1 => Signed::Min,
			_ => Signed::Pos,
		};
		out.push(wide as u8);
		out.push(wide as i8 as u8);
		out.extend_from_slice(&(wide as i16).to_be_bytes());
		out.extend_from_slice(&(wide as u64).to_be_bytes());
		out.push(signed as u8);
		out.extend_from_slice(&(signed as u16).to_be_bytes());
		out.extend_from_slice(&(signed as u32).to_be_bytes());
		out.extend_from_slice(&(signed as i64).to_be_bytes());
		out.extend_from_slice(&(signed as u128).to_be_bytes());
	}
}

/// Loops: labeled, `while let`, ranges, values out of `break`, loops that end a function,
/// and values of type `!`.
pub fn loops(data: &[u8], out: &mut Vec<u8>) {
	let mut stack: Vec<u8> = data.to_vec();
	let mut popped = 0u32;
	while let Some(top) = stack.pop() {
		if top == 0 {
			continue;
		}
		popped += top as u32;
		if popped > 1000 {
			break;
		}
	}
	out.extend_from_slice(&popped.to_be_bytes());

	let mut found = None;
	'outer: for i in 0..data.len() {
		for j in (i + 1)..data.len() {
			if data[i] == data[j] {
				found = Some((i, j));
				break 'outer;
			}
			if data[j] == 0 {
				continue 'outer;
			}
		}
	}
	match found {
		Some((i, j)) => out.extend_from_slice(&[i as u8, j as u8]),
		None => out.push(0xff),
	}

	let mut n = data.len() as u64;
	let steps = loop {
		if n <= 1 {
			break 0u8;
		}
		n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
		if n > 100 {
			break 1;
		}
	};
	out.push(steps);

	let mut count = 0u8;
	for _ in 0..=3u8 {
		count += 1;
	}
	let mut i = 10i8;
	while i > -3 {
		i -= 4;
		count += 1;
	}
	out.push(count);

	// Ranges stepped by hand: signed across zero, up to the type's end, and an item read
	// after the range changed.
	let lo = -((data.len() % 4) as i32);
	let mut r = lo..lo + 3;
	let first = r.next();
	r.end = r.start;
	let second = r.next();
	if let Some(x) = first {
		out.extend_from_slice(&x.to_be_bytes());
	}
	if let Some(x) = second {
		out.extend_from_slice(&x.to_be_bytes());
	}
	for k in lo..2 {
		out.push(k as u8);
	}
	for k in 124 - lo as i8..i8::MAX {
		out.push(k as u8);
	}

	out.push(first_repeat(data));
	out.extend_from_slice(&tail_break(data).to_be_bytes());
	out.push(arm_loop(data));
	out.extend_from_slice(&(position(data, &0) as u32).to_be_bytes());
	let settle = |mut x: u8| -> u8 {
		loop {
			if x < 10 {
				return x;
			}
			x /= 3;
		}
	};
	out.push(settle(data.len() as u8 * 7));
	drain(data, out);
	out.push(returned_loop(data));
	branch_loops(data, out);
	diverging(data, out);
}

/// Values of type `!` where a value goes, triggered by the first two bytes so most
/// inputs run every case to its end.
fn diverging(data: &[u8], out: &mut Vec<u8>) {
	let head = &data[..data.len().min(2)];
	let b = head.first().copied().unwrap_or(0);
	out.push(returned_never(b, data.len()));
	out.push(broken_never(head));
	let check = |b: u8| -> u8 {
		if b == 0xfd {
			#[allow(unreachable_code, clippy::needless_return, reason = "a `return` of a `!` value")]
			return panic!("closure {b}");
		}
		b / 3
	};
	out.push(check(b));
	out.push(guarded_return(head));
	let found = bound_never(head, out);
	out.push(found);
	out.push(closure_never(b));
	generic_never(b, data.len());
}

/// Calls that never return, through a closure, a function pointer, and a closure a
/// function made: a statement, a bound value, and the tail.
fn closure_never(b: u8) -> u8 {
	let fail = |b: u8| -> ! { panic!("closure failed on {b:#x}") };
	let ptr: fn(u8) -> ! = bail;
	let made = made_never();
	if b == 0xf5 {
		fail(b);
	}
	let x: u8 = if b == 0xf4 { ptr(b) } else { b / 5 };
	match b {
		0xf3 => fail(b),
		0xfa => made(b),
		_ => x,
	}
}

/// A closure that never returns, made by a function.
fn made_never() -> impl Fn(u8) -> ! {
	|b| panic!("made {b}")
}

/// Generic functions instantiated at `!`, which edition 2024 infers for a value that
/// never exists: given one, and given an `Option` of none, which then calls one.
#[allow(unreachable_code, clippy::diverging_sub_expression, reason = "`!` arguments")]
fn generic_never(b: u8, len: usize) {
	fn pass<T>(x: T) -> T {
		x
	}
	fn unwrap_or_bail<T>(o: Option<T>, b: u8) -> T {
		match o {
			Some(x) => pass(x),
			None => bail(b),
		}
	}
	if b == 0xf1 {
		pass(bail(b));
	}
	if b == 0xf6 {
		pass(panic!("passed {b}"));
	}
	if b == 0xf9 {
		let o = if len > 1 { None } else { Some(panic!("some {b}")) };
		unwrap_or_bail(o, b);
	}
}

/// A function that never returns.
fn bail(b: u8) -> ! {
	panic!("failed on {b:#x}")
}

/// `return`, and `break` out of the function's labeled block, with values that never exist.
#[allow(unreachable_code, clippy::needless_return, reason = "`return` of `!` values")]
fn returned_never(b: u8, len: usize) -> u8 {
	match b {
		0xf7 => return panic!("returned {b}"),
		0xf8 => return if len % 2 == 0 { panic!("then {b}") } else { bail(b) },
		_ => {}
	}
	'value: {
		if b == 0xfe {
			break 'value bail(b);
		}
		b ^ 0x11
	}
}

/// A `return` of a `match` that guards, whose arms fall through to the next.
#[allow(clippy::needless_return, reason = "the `return` is not the function's tail")]
fn guarded_return(data: &[u8]) -> u8 {
	if !data.is_empty() {
		return match data {
			[a, b] if a > b => a - b,
			[a, ..] if *a > 0x80 => 1,
			_ => 2,
		};
	}
	3
}

/// A loop ending the function, left by `break` with values that never exist.
#[allow(unreachable_code, reason = "`break` with `!` values")]
fn broken_never(data: &[u8]) -> u8 {
	let mut i = 0;
	loop {
		if i >= data.len() {
			return 0xcc;
		}
		match data[i] {
			0xfb => break panic!("broken at {i}"),
			0xfc => break bail(data[i]),
			0 => return i as u8,
			_ => i += 1,
		}
	}
}

/// Values that never exist where a value is needed: bound, and passed. A loop that
/// only `return` leaves gives the function's result.
#[allow(unreachable_code, clippy::diverging_sub_expression, reason = "`!` values")]
fn bound_never(data: &[u8], out: &mut Vec<u8>) -> u8 {
	let mut i = 0;
	let found: u8 = loop {
		if i >= data.len() {
			return 0xbb;
		}
		match data[i] {
			0xef => {
				let x: u8 = panic!("bound at {i}");
				out.push(x);
			}
			0xf0 => out.push(bail(data[i])),
			0 => out.push(return i as u8),
			0x80 => out.push(loop {
				i += 1;
				if i >= data.len() || data[i] < 0x80 {
					return i as u8;
				}
			}),
			_ => {}
		}
		i += 1;
	};
	found
}

/// Loops that `break` ending a guarded `match` arm and an `if let` chain branch.
fn branch_loops(data: &[u8], out: &mut Vec<u8>) {
	let mut i = 0;
	while i + 1 < data.len() {
		let w = &data[i..i + 2];
		i += 2;
		match w {
			[a, b] if a > b => {
				let mut n = *a;
				loop {
					if n <= *b {
						break;
					}
					n -= 1;
					out.push(n);
				}
			}
			[a, ..] => out.push(*a),
			_ => {}
		}
		if let [a, b] = w
			&& a % 3 == b % 3
		{
			let mut n = *a;
			loop {
				n /= 2;
				out.push(n);
				if n < 4 {
					break;
				}
			}
		} else {
			out.push(0xaa);
		}
	}
}

/// A loop that only `return` leaves, returned itself, around loops that `break`.
#[allow(unreachable_code, clippy::needless_return)]
fn returned_loop(data: &[u8]) -> u8 {
	let mut i = 0;
	return loop {
		if i >= data.len() {
			return 0xdd;
		}
		for &b in &data[i..] {
			if b == 0 {
				break;
			}
			if b > 0xf0 {
				return b ^ i as u8;
			}
		}
		i += 1;
	};
}

/// A loop that ends the function, left only by `return` from nested scopes.
fn first_repeat(data: &[u8]) -> u8 {
	let mut i = 0;
	'scan: loop {
		if i >= data.len() {
			return 0xff;
		}
		let mut j = i + 1;
		while j < data.len() {
			if data[j] == 0 {
				i = j + 1;
				continue 'scan;
			}
			match data[i] == data[j] {
				true => return data[j] ^ i as u8,
				false => {}
			}
			j += 1;
		}
		i += 1;
	}
}

/// A loop that ends the function, left by `break` with a value from nested scopes.
fn tail_break(data: &[u8]) -> u16 {
	let mut i = 0;
	'outer: loop {
		if i >= data.len() {
			break 0x100;
		}
		let b = data[i];
		i += 1;
		match b % 4 {
			0 => continue,
			1 => {
				for &c in &data[i..] {
					if c == b {
						break 'outer c as u16 * 2;
					}
				}
			}
			2 if b > 100 => break b as u16 + i as u16,
			_ => {
				let mut n = b;
				loop {
					if n < 10 {
						break;
					}
					n /= 3;
					if n == 7 {
						break 'outer 7;
					}
				}
			}
		}
	}
}

/// A loop that only `return` leaves, in a `match` arm that needs a value.
fn arm_loop(data: &[u8]) -> u8 {
	let mut k = 0;
	let start = match data.first() {
		Some(&b) if b >= 0x80 => b,
		_ => loop {
			if k >= data.len() {
				return 0xee;
			}
			if data[k] >= 0x80 {
				return data[k] - 0x80;
			}
			k += 1;
		},
	};
	start / 2
}

/// A generic function ending in a loop.
fn position<T: PartialEq>(items: &[T], want: &T) -> usize {
	let mut i = 0;
	loop {
		if i == items.len() || items[i] == *want {
			return i;
		}
		i += 1;
	}
}

/// A unit function ending in a loop.
fn drain(data: &[u8], out: &mut Vec<u8>) {
	let mut i = 0;
	loop {
		if i >= data.len() {
			return;
		}
		out.push(data[i] ^ 0x5a);
		i += 2;
	}
}

/// Patterns: slices, ranges, guards that bind, `@` bindings, and let chains.
pub fn patterns(data: &[u8], out: &mut Vec<u8>) {
	match data {
		[] => out.push(0),
		[one] => out.push(*one),
		[first, .., last] if first == last => out.push(2),
		[first, second, rest @ ..] => {
			out.push(*first ^ *second);
			out.push(rest.len() as u8);
		}
	}
	for &b in &data[..data.len().min(4)] {
		let class = match b {
			0 => 0,
			1..=9 => 1,
			10 | 20 | 30 => 2,
			n @ 100..200 if n % 2 == 0 => 3,
			n if n >= 250 => 4,
			_ => 5,
		};
		out.push(class);
	}
	if let [a, b, ..] = data
		&& let Some(d) = dir(*a)
		&& *b > 10
	{
		out.push(u8::from(d));
	} else {
		out.push(0xee);
	}
	let shape = shape(data);
	let area = match shape {
		Shape::Rect { width: 0, .. } | Shape::Dot => 0,
		Shape::Rect {
			corner: Point { x, .. },
			width,
			..
		} if x > 0 => width as i64 * x as i64,
		Shape::Rect { width, height, .. } => -(width as i64 * height as i64),
		Shape::Line(a, b) => (a.x - b.x) as i64 * (a.y - b.y) as i64,
	};
	out.extend_from_slice(&area.to_be_bytes());
	// One binding from either side of an or-pattern.
	let either: Result<u8, u8> = if data.len() % 2 == 0 {
		Ok(data.len() as u8)
	} else {
		Err(0xa0)
	};
	match either {
		Ok(v) | Err(v) => out.push(v),
	}
	// An empty repeat, which MIR never assigns since it holds nothing.
	let none = [data.len() as u8; 0];
	out.push(none.len() as u8 + none.iter().sum::<u8>());
	// A rest pattern borrowing bytes changes them where they are.
	let mut bytes = [data.len() as u8, 1, 2];
	let [_, tail @ ..] = &mut bytes;
	tail[0] = 9;
	out.extend_from_slice(&bytes);
	let total = total(&[shape, Shape::Dot], out) + total(&[Meters(3), Meters(u64::MAX)], out);
	out.extend_from_slice(&total.to_be_bytes());

	// An `Err` of `Void` never exists, so its arm matches nothing, or is left out.
	let parsed: Result<u8, Void> = Ok(data.len() as u8);
	let a = match parsed {
		Ok(v) => v,
		Err(e) => absurd((e, 1)),
	};
	let b = match parsed {
		Ok(v) => v,
		Err(e) => match e {},
	};
	let c = match parsed {
		Ok(v) => v,
	};
	out.extend_from_slice(&[a, b, c, early(a)]);
	out.extend_from_slice(format!("{parsed:?}").as_bytes());
}

/// An enum without variants: no value of it exists.
#[derive(Clone, Copy, Debug)]
pub enum Void {}

/// A value holding a `Void` can't exist either.
fn absurd(v: (Void, u8)) -> u8 {
	match v {}
}

/// A `match` without arms on a scrutinee that diverges, not on an empty type.
#[allow(unreachable_code)]
fn early(n: u8) -> u8 {
	match return n.wrapping_add(1) {}
}

/// Closures capturing by reference and by move, and functions as values.
pub fn closures(data: &[u8], out: &mut Vec<u8>) {
	let mut sum = 0u32;
	let mut add = |v: u8| sum += v as u32;
	for &b in data {
		add(b);
	}
	out.extend_from_slice(&sum.to_be_bytes());

	let base = data.len() as u32;
	let scale = move |v: u32| v * 3 + base;
	out.extend_from_slice(&apply(scale, sum).to_be_bytes());
	// A `move` closure copies what it captured, which a later change to the original misses.
	let mut n = base;
	let copied = move || n + 1;
	n += 10;
	out.extend_from_slice(&(copied() + n).to_be_bytes());

	// A closure writing a whole array through a reference it captured changes the caller's.
	let mut whole = [data.len() as u8];
	overwrite(&mut whole);
	out.push(whole[0]);

	let mut parsed: Vec<Option<Dir>> = Vec::new();
	for &b in data {
		parsed.push(dir(b));
	}
	let mut tags = Vec::new();
	for d in &parsed {
		tags.push(d.map(u8::from).unwrap_or(0));
	}
	out.extend_from_slice(&tags);
}

/// Replaces `a` whole from a closure.
fn overwrite(a: &mut [u8; 1]) {
	let mut write = || *a = [9];
	write();
}

fn apply<F: Fn(u32) -> u32>(f: F, v: u32) -> u32 {
	f(v)
}

/// Options, results, `?`, and strings.
pub fn library(data: &[u8], out: &mut Vec<u8>) {
	let checked = |a: u8, b: u8| -> Option<u8> {
		let sum = a.checked_add(b)?;
		let doubled = sum.checked_mul(2)?;
		Some(doubled - 1)
	};
	let a = data.first().copied().unwrap_or(0);
	let b = data.get(1).copied().unwrap_or(0);
	out.push(checked(a, b).unwrap_or(0xff));

	let parsed: Result<u8, &str> = match a {
		0 => Err("zero"),
		n => Ok(n),
	};
	let mapped = parsed.map(|n| n / 2).map_err(|e| e.len() as u8);
	match mapped {
		Ok(n) => out.extend_from_slice(&[0, n]),
		Err(n) => out.extend_from_slice(&[1, n]),
	}

	// `?` in a function that returns a `Poll`, converting the error.
	let ready = |a: u8| -> std::task::Poll<Result<u8, u8>> {
		let half: Result<u8, Dir> = match dir(a) {
			Some(d) if a % 2 == 1 => Err(d),
			_ => Ok(a / 2),
		};
		let n = half?;
		std::task::Poll::Ready(Ok(n))
	};
	match ready(b) {
		std::task::Poll::Ready(Ok(n)) => out.extend_from_slice(&[2, n]),
		std::task::Poll::Ready(Err(e)) => out.extend_from_slice(&[3, e]),
		std::task::Poll::Pending => out.push(4),
	}

	match String::from_utf8(data.to_vec()) {
		Ok(s) => {
			out.push(1);
			out.extend_from_slice(&(s.len() as u32).to_be_bytes());
			out.extend_from_slice(s.as_bytes());
		}
		Err(_) => out.push(0),
	}
}

/// Steps a total by `x`: breaks with what it can't take, or continues with the new total.
fn step(total: u16, x: u8) -> ControlFlow<Option<u8>, u16> {
	match total.checked_add(u16::from(x) * 0x40) {
		_ if x == 0 => ControlFlow::Break(None),
		Some(n) if n > 0x2000 => ControlFlow::Break(Some(x)),
		Some(n) => ControlFlow::Continue(n),
		None => ControlFlow::Break(Some(0xff)),
	}
}

/// A step that may be pending, as a polled read is.
fn polled(x: u8) -> Poll<Result<ControlFlow<()>, u8>> {
	match x % 4 {
		0 => Poll::Pending,
		1 => Poll::Ready(Err(x)),
		2 => Poll::Ready(Ok(ControlFlow::Break(()))),
		_ => Poll::Ready(Ok(ControlFlow::Continue(()))),
	}
}

/// `ControlFlow`s made by hand rather than by `?`, and a value replaced through a
/// `MutexGuard`.
pub fn flows(data: &[u8], out: &mut Vec<u8>) {
	let mut total = 0u16;
	for &x in data {
		let flow = step(total, x);
		out.extend_from_slice(&[flow.is_break() as u8, flow.is_continue() as u8]);
		match flow {
			ControlFlow::Continue(n) => total = n,
			ControlFlow::Break(Some(b)) => out.push(b),
			ControlFlow::Break(None) => out.push(0xfe),
		}
		match polled(x) {
			Poll::Pending => out.push(0),
			Poll::Ready(Err(e)) => out.push(e),
			Poll::Ready(Ok(flow)) => out.push(2 + flow.is_break() as u8),
		}
	}
	out.extend_from_slice(&total.to_be_bytes());

	// The two shapes `?` branches on, made by hand.
	let a = data.first().copied().unwrap_or(0);
	let result: ControlFlow<Result<Infallible, u8>, u16> = match a % 3 {
		0 => ControlFlow::Break(Err(a)),
		_ => ControlFlow::Continue(u16::from(a) + 0x100),
	};
	out.push(result.is_continue() as u8);
	match result {
		ControlFlow::Continue(n) => out.extend_from_slice(&n.to_be_bytes()),
		ControlFlow::Break(Err(e)) => out.push(e),
	}
	let option: ControlFlow<Option<Infallible>, u8> = match a % 5 {
		0 => ControlFlow::Break(None),
		_ => ControlFlow::Continue(a / 5),
	};
	out.push(option.is_break() as u8);
	match option {
		ControlFlow::Continue(n) => out.push(n),
		ControlFlow::Break(None) => out.push(0xfd),
	}

	let slot = Mutex::new(None);
	for &x in data {
		let mut held = slot.lock().unwrap();
		if !matches!(*held, Some(h) if h >= x) {
			*held = Some(x);
		}
	}
	out.push(slot.lock().unwrap().unwrap_or(0xfc));
}
