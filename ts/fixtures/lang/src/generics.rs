//! Methods on generic structs and enums, each instance its own method.

use crate::fmt::{Pair, put};
use crate::shapes::{Area, Meters, Point, Shape};

/// A generic struct, with methods on every instance, on some, and on one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wrap<T> {
	pub value: T,
	pub count: u8,
}

impl<T> Wrap<T> {
	pub fn new(value: T) -> Self {
		Self { value, count: 0 }
	}

	pub fn get(&self) -> &T {
		&self.value
	}

	pub fn bump(&mut self) -> u8 {
		self.count = self.count.wrapping_add(1);
		self.count
	}

	pub fn into_inner(self) -> T {
		self.value
	}

	/// A method with its own type parameter.
	pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Wrap<U> {
		Wrap {
			value: f(self.value),
			count: self.count,
		}
	}
}

impl<T: Area> Wrap<T> {
	pub fn scaled(&self) -> u64 {
		self.value.area().saturating_add(self.count as u64)
	}
}

impl Wrap<u8> {
	pub fn double(&self) -> u16 {
		self.value as u16 * 2
	}
}

impl<T: Area> Area for Wrap<T> {
	const SIDES: u8 = T::SIDES;

	fn area(&self) -> u64 {
		self.value.area()
	}
}

/// A generic enum, with a companion function per instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Either<L, R> {
	Left(L),
	Right { value: R },
	Neither,
}

impl<L: Copy, R: Copy> Either<L, R> {
	pub fn left(&self) -> Option<L> {
		match *self {
			Either::Left(l) => Some(l),
			_ => None,
		}
	}

	pub fn flip(self) -> Either<R, L> {
		match self {
			Either::Left(l) => Either::Right { value: l },
			Either::Right { value } => Either::Left(value),
			Either::Neither => Either::Neither,
		}
	}
}

/// Two parameters, one nested in another generic type.
#[derive(Debug)]
pub struct Tagged<K, V> {
	pub key: K,
	pub inner: Wrap<V>,
	pub all: Vec<Wrap<V>>,
}

impl<K: Copy, V: Copy> Tagged<K, V> {
	fn first(&self) -> Option<Wrap<V>> {
		self.all.first().copied()
	}
}

/// Each method once per instance: `Wrap` of a struct, an enum, a number, and itself.
pub fn methods(data: &[u8], out: &mut Vec<u8>) {
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	let mut point = Wrap::new(Point {
		x: at(0) as i32,
		y: -2,
	});
	point.bump();
	let mut byte = Wrap::new(at(1));
	byte.bump();
	byte.bump();
	let nested = Wrap::new(Wrap::new(Pair(at(2), -3)));
	let meters = Wrap {
		value: Meters(at(3) as u64),
		count: at(4),
	};
	let shape = Wrap::new(Shape::Rect {
		corner: point.into_inner(),
		width: at(5) as u16,
		height: 3,
	});
	put(out, &format!("{point:?} {byte:?} {nested:?} {shape:?}"));
	put(out, &format!("{nested:#?}"));
	out.extend_from_slice(&byte.double().to_be_bytes());
	out.push(*byte.get());
	out.push(nested.get().get().0);
	out.extend_from_slice(&meters.scaled().to_be_bytes());
	out.extend_from_slice(&shape.scaled().to_be_bytes());
	meters.describe(out);
	shape.describe(out);
	let mapped = byte.map(|b| Pair(b, at(6) as i64));
	put(out, &format!("{mapped:?} {:?}", mapped == Wrap::new(Pair(at(1), 0))));

	let eithers: [Either<u8, Point>; 3] = [
		Either::Left(at(7)),
		Either::Right { value: Point::new(1, -1) },
		Either::Neither,
	];
	for either in eithers {
		let flipped = either.flip();
		put(
			out,
			&format!("{either:?} {flipped:#?} {:?} {:?}", either.left(), flipped.left()),
		);
	}
	// Literals TypeScript would widen, as a class parameter infers from them.
	let mut list = Wrap {
		value: Vec::new(),
		count: 1,
	};
	list.value.push(at(10) as u16);
	list.bump();
	let none = Wrap {
		value: None::<u8>,
		count: 0,
	};
	let some = Wrap::new(data.get(11).copied());
	let err: Wrap<Result<u8, Shape>> = Wrap {
		value: Err(Shape::Dot),
		count: 2,
	};
	put(
		out,
		&format!("{list:?} {none:?} {some:?} {err:?} {:?}", none.get().unwrap_or(some.count)),
	);

	let tagged = Tagged {
		key: Either::<u8, u16>::Right { value: 500 },
		inner: Wrap::new(at(8) as i16 - 100),
		all: [Wrap::new(-1), Wrap::new(at(9) as i16)].to_vec(),
	};
	put(out, &format!("{tagged:?} {:?}", tagged.first()));
	out.extend_from_slice(&tagged.inner.value.to_be_bytes());
	out.push(tagged.key.left().unwrap_or(0));
}
