//! Trait objects of the crate's own traits, behind `&`, `&mut`, and `Box`: each call runs
//! the impl of the value's own type, a struct, an enum, or a number, changing it in place
//! through `&mut self` wherever it is, a provided method, a supertrait's, `Debug` among
//! them, `self: Box<Self>`, and an upcast to the supertrait.

use std::fmt::{self, Debug};

use crate::fmt::put;

trait Shape {
	fn area(&self) -> u32;
	fn grow(&mut self, by: u8);
	/// Provided, and overridden by one impl.
	fn sides(&self) -> u8 {
		4
	}
	fn into_area(self: Box<Self>) -> u32 {
		self.area().wrapping_add(1)
	}
	/// Over early-bound lifetimes, which its signature takes as arguments too.
	fn larger<'a: 'b, 'b>(&'a self, other: &'b u32) -> u32 {
		self.area().max(*other)
	}
	/// Writes through a `&mut` to a number, which is an `rt.Place`.
	fn tally(&self, sum: &mut u32) {
		*sum = sum.wrapping_add(self.area());
	}
}

trait Named: Shape + Debug {
	fn label(&self) -> String;
}

#[derive(Debug)]
struct Square {
	side: u16,
}

impl Shape for Square {
	fn area(&self) -> u32 {
		u32::from(self.side) * u32::from(self.side)
	}
	fn grow(&mut self, by: u8) {
		self.side = self.side.wrapping_add(u16::from(by));
	}
}

impl Named for Square {
	fn label(&self) -> String {
		format!("square {}", self.side)
	}
}

/// An enum, which JS copies, so `&mut self` reaches it through a place.
#[derive(Debug, Clone, Copy)]
enum Tri {
	Flat,
	Tall(u8),
}

impl Shape for Tri {
	fn area(&self) -> u32 {
		match self {
			Tri::Flat => 0,
			Tri::Tall(h) => u32::from(*h) * 3,
		}
	}
	fn grow(&mut self, by: u8) {
		*self = match *self {
			Tri::Flat if by % 2 == 0 => Tri::Flat,
			Tri::Flat => Tri::Tall(by),
			Tri::Tall(h) => Tri::Tall(h.wrapping_add(by)),
		};
	}
	fn sides(&self) -> u8 {
		3
	}
	fn into_area(self: Box<Self>) -> u32 {
		self.area().wrapping_mul(2)
	}
}

impl Named for Tri {
	fn label(&self) -> String {
		format!("tri {self:?}")
	}
}

/// A number, which JS copies too.
impl Shape for u32 {
	fn area(&self) -> u32 {
		*self
	}
	fn grow(&mut self, by: u8) {
		*self = self.wrapping_mul(u32::from(by));
	}
}

/// Writes a `&dyn Debug` by calling its `fmt`, not through a placeholder.
struct Shown<'a>(&'a dyn Debug);

impl Debug for Shown<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

fn total(shapes: &[&dyn Shape]) -> u32 {
	shapes.iter().fold(0, |sum, s| sum.wrapping_add(s.area()))
}

fn grow_twice(s: &mut dyn Shape, by: u8) {
	s.grow(by);
	s.grow(by.wrapping_add(1));
}

/// Over a type parameter that the trait object instantiates.
fn sides_of<T: Shape + ?Sized>(s: &T) -> u8 {
	s.sides()
}

fn upcast(n: &dyn Named) -> &dyn Shape {
	n
}

fn make(kind: u8, n: u8) -> Box<dyn Named> {
	match kind % 2 {
		0 => Box::new(Square { side: u16::from(n) }),
		_ => Box::new(if n % 3 == 0 { Tri::Flat } else { Tri::Tall(n) }),
	}
}

pub fn dyns(data: &[u8], out: &mut Vec<u8>) {
	let [a, b, c, rest @ ..] = data else {
		return;
	};
	let (a, b, c) = (*a, *b, *c);

	// Borrowed, of each type.
	let mut sq = Square { side: u16::from(a) };
	let mut tri = if b % 2 == 0 { Tri::Flat } else { Tri::Tall(b) };
	let mut n = u32::from(c);
	put(out, &format!("{} ", total(&[&sq, &tri, &n])));

	// Changed in place through `&mut dyn`, a local each.
	grow_twice(&mut sq, b);
	grow_twice(&mut tri, c);
	grow_twice(&mut n, a);
	put(out, &format!("{} {tri:?} {n} ", sq.side));
	put(out, &format!("{} {} ", sides_of::<dyn Shape>(&tri), sides_of(&sq)));

	// Owned, changed through the box, then consumed by `self: Box<Self>`.
	let mut shapes: Vec<Box<dyn Named>> = Vec::new();
	for (i, &x) in rest.iter().enumerate() {
		shapes.push(make(x.wrapping_add(i as u8), x));
	}
	for i in 0..shapes.len() {
		let s = &mut shapes[i];
		if i % 2 == 0 {
			s.grow(a);
		}
		put(
			out,
			&format!("{}:{}:{}:{:?} ", s.label(), s.area(), sides_of(upcast(&**s)), s),
		);
	}
	let mut sum = 0u32;
	for s in &shapes {
		s.tally(&mut sum);
	}
	put(
		out,
		&format!("{sum} {:?} {} ", Shown(&tri), shapes[0].larger(&u32::from(b))),
	);
	for s in shapes {
		sum = sum.wrapping_add(s.into_area());
	}
	let boxed: Box<dyn Shape> = Box::new(n);
	put(out, &format!("{sum} {}", boxed.into_area()));
}
