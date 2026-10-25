//! Control flow, patterns, aggregates, closures, and traits.

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

	match String::from_utf8(data.to_vec()) {
		Ok(s) => {
			out.push(1);
			out.extend_from_slice(&(s.len() as u32).to_be_bytes());
			out.extend_from_slice(s.as_bytes());
		}
		Err(_) => out.push(0),
	}
}
