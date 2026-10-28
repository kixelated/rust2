//! Trait objects of the crate's own traits, behind `&`, `&mut`, and `Box`: each call runs
//! the impl of the value's own type, a struct, an enum, or a number, changing it in place
//! through `&mut self` wherever it is, a provided method, a supertrait's, `Debug` among
//! them, `self: Box<Self>`, and an upcast to the supertrait. Behind an `Arc`, the object
//! shares the count of the `Arc` it was made from.

use std::fmt::{self, Debug};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Weak};

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

/// Counts its calls, which every `Arc` of it sees.
struct Hits(AtomicU32);

impl Shape for Hits {
	fn area(&self) -> u32 {
		self.0.fetch_add(1, Ordering::Relaxed)
	}
	fn grow(&mut self, by: u8) {
		*self.0.get_mut() = self.0.get_mut().wrapping_add(u32::from(by));
	}
}

fn counts<T: ?Sized>(arcs: &[&Arc<T>]) -> String {
	let counts: Vec<String> = arcs.iter().map(|a| Arc::strong_count(a).to_string()).collect();
	counts.join(",")
}

pub fn shared(data: &[u8], out: &mut Vec<u8>) {
	let [a, b, rest @ ..] = data else {
		return;
	};

	// Two objects of one `Arc`, and an upcast of each: one count, which either side's
	// clones and drops change, and one value.
	let sq = Arc::new(Square { side: u16::from(*a) });
	let named: Arc<dyn Named> = sq.clone();
	let again: Arc<dyn Named> = Arc::<Square>::clone(&sq);
	let shape: Arc<dyn Shape> = named.clone();
	let up: Arc<dyn Shape> = again.clone();
	put(out, &format!("{} ", counts(&[&sq])));
	put(
		out,
		&format!("{} {} {} {:?} ", named.label(), shape.area(), sides_of(&*up), &*named),
	);
	let (weak, other) = (Arc::downgrade(&shape), Arc::downgrade(&up));
	put(
		out,
		&format!(
			"{} {} {} ",
			Arc::ptr_eq(&named, &again),
			Arc::ptr_eq(&shape, &up),
			Weak::ptr_eq(&weak, &other)
		),
	);
	drop(sq);
	drop(named);
	put(out, &format!("{} {} ", counts(&[&again]), Weak::strong_count(&weak)));
	let upgraded = weak.upgrade().map(|s| s.area());
	drop(again);
	put(out, &format!("{} ", counts(&[&up])));
	drop(up);
	put(
		out,
		&format!("{} {} {:?} ", counts(&[&shape]), weak.upgrade().is_some(), upgraded),
	);
	drop(shape);
	put(
		out,
		&format!("{} {} ", weak.upgrade().is_none(), Weak::strong_count(&other)),
	);

	// Calls through every `Arc` reach the one value.
	let hits = Arc::new(Hits(AtomicU32::new(u32::from(*b))));
	let objects: Vec<Arc<dyn Shape>> = (0..3).map(|_| hits.clone() as Arc<dyn Shape>).collect();
	let sum = objects.iter().fold(0u32, |sum, s| sum.wrapping_add(s.area()));
	put(
		out,
		&format!("{sum} {} {} ", hits.0.load(Ordering::Relaxed), counts(&[&hits])),
	);
	drop(objects);
	put(out, &format!("{} ", counts(&[&hits])));

	// Of a number and an enum, which JS copies, and two `Arc`s that are not the same.
	let n: Arc<dyn Shape> = Arc::new(u32::from(*b));
	let m: Arc<dyn Shape> = Arc::new(u32::from(*b));
	put(out, &format!("{} {} ", n.area(), Arc::ptr_eq(&n, &m)));
	let tris: Vec<Arc<dyn Named>> = rest
		.iter()
		.map(|&x| match x % 3 {
			0 => Arc::new(Tri::Flat) as Arc<dyn Named>,
			1 => Arc::new(Tri::Tall(x)),
			_ => Arc::new(Square { side: u16::from(x) }),
		})
		.collect();
	let kept: Vec<Arc<dyn Named>> = tris.iter().step_by(2).cloned().collect();
	for t in &tris {
		put(out, &format!("{}:{}:{} ", t.label(), t.area(), Arc::strong_count(t)));
	}
	drop(tris);
	let mut total = 0u32;
	for k in &kept {
		k.tally(&mut total);
	}
	put(out, &format!("{total} {}", counts(&kept.iter().collect::<Vec<_>>())));
}
