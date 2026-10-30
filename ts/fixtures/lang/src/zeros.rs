//! Zero-sized values, which MIR never assigns and reads as constants: unit structs, empty
//! arrays and arrays of zero-sized elements, structs of zero-sized fields, enums of one
//! variant, `PhantomData`, closures that capture nothing, and std's iterators made of
//! them, as results, as values, in collections, and as they drop.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Unit;

/// Of zero-sized fields, so zero-sized too.
#[derive(Clone, Copy)]
struct Pair(Unit, PhantomData<u8>);

/// Of one variant, so zero-sized.
#[derive(Clone, Copy, Debug, PartialEq)]
enum One {
	Only,
}

/// A closure it holds, zero-sized when the closure captures nothing.
struct Holds<F>(F);

/// What dropped, four bits each, the latest lowest.
static LOG: AtomicU32 = AtomicU32::new(0);

fn log(tag: u32) {
	LOG.store(LOG.load(Relaxed) << 4 | tag, Relaxed);
}

struct Loud;

impl Drop for Loud {
	fn drop(&mut self) {
		log(1);
	}
}

struct Quiet;

impl Drop for Quiet {
	fn drop(&mut self) {
		log(2);
	}
}

/// Drops itself, then its fields in order.
struct Both(Loud, Quiet);

impl Drop for Both {
	fn drop(&mut self) {
		log(3);
	}
}

fn unit() -> Unit {
	Unit
}

fn pair() -> Pair {
	Pair(Unit, PhantomData)
}

fn none<T>() -> [T; 0] {
	[]
}

fn units() -> [Unit; 3] {
	[Unit; 3]
}

fn one() -> One {
	One::Only
}

fn tuple() -> ((), Unit, [u8; 0]) {
	((), Unit, [])
}

fn phantom<T>() -> PhantomData<T> {
	PhantomData
}

fn adder() -> impl Fn(u8) -> u8 {
	|b| b.wrapping_add(1)
}

fn loud() -> Loud {
	Loud
}

fn made<T: Default>() -> T {
	T::default()
}

/// The case `kind` picks, on `data`; each puts what it read.
pub fn zeros(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	LOG.store(0, Relaxed);
	let n = data.len();
	match kind % 7 {
		// Derived and written impls whose results nothing assigns.
		0 => {
			let u = Unit;
			let c = u.clone();
			let p = pair().clone();
			let q: Unit = made();
			let _: PhantomData<u8> = phantom();
			out.push(u8::from(c == u) | u8::from(p.0 == q) << 1);
			out.extend(format!("{c:?} {:?}", One::Only.clone()).bytes());
		}
		// Closures that return them, read as items.
		1 => {
			out.push(data.iter().map(|_| [0u8; 0]).count() as u8);
			out.push(data.iter().map(|_| Unit).collect::<Vec<Unit>>().len() as u8);
			out.push(data.iter().map(|_| [Unit; 3]).flatten().count() as u8);
			out.push(
				data.iter()
					.map(|_| pair())
					.filter(|p| p.0 == Pair(Unit, PhantomData).0)
					.count() as u8,
			);
			let f = |_: u8| One::Only;
			out.push(u8::from(f(1) == one()));
		}
		2 => {
			let s = format!(
				"{:?} {:?} {:?} {:?} {:?}",
				unit(),
				none::<String>(),
				units(),
				one(),
				tuple()
			);
			out.extend(s.bytes());
		}
		// Closures that capture nothing as values: returned, moved, and held in a field.
		3 => {
			let f = adder();
			let g = adder();
			let h = g;
			let held = Holds(|b: u8| b ^ 0x5a);
			out.extend(data.iter().map(|&b| f(b)));
			out.extend(data.iter().copied().map(h));
			out.extend(data.iter().copied().map(adder()));
			out.extend(data.iter().map(|&b| (held.0)(b)));
		}
		// Drops, in order: of locals, a result, a `Vec`'s items, a struct's fields after it,
		// a moved value, and an array's items, but not of what's forgotten.
		4 => {
			{
				let _a = Loud;
				let _b = Quiet;
			}
			drop(loud());
			let v: Vec<Loud> = data.iter().map(|_| Loud).collect();
			out.push(v.len() as u8);
			drop(v);
			drop(Both(Loud, Quiet));
			std::mem::forget(Loud);
			let x = Quiet;
			let y = x;
			drop(y);
			drop([Quiet, Quiet]);
			out.extend(LOG.load(Relaxed).to_be_bytes());
		}
		5 => {
			let mut v: Vec<()> = Vec::with_capacity(n);
			v.extend(data.iter().map(|_| ()));
			out.push(vec![Unit; n].len() as u8);
			out.push(v.len() as u8);
		}
		// std's iterators of zero-sized fields, beside one that captures.
		_ => {
			out.push(std::iter::from_fn(|| None::<u8>).count() as u8);
			out.push(std::iter::from_fn(|| Some(Unit)).take(n).count() as u8);
			let mut left = n;
			out.extend(std::iter::from_fn(move || {
				left = left.checked_sub(1)?;
				Some(left as u8)
			}));
			let empty = std::iter::empty::<u8>().map(|b| b + 1);
			out.push(empty.size_hint().0 as u8);
			out.push(empty.count() as u8);
			out.push(std::iter::empty::<&u8>().copied().rev().filter(|_| true).count() as u8);
		}
	}
}
