pub const LIMIT: u32 = BASE + 1;
const BASE: u32 = 4;

pub trait Shape {
	fn area(&self) -> u32;
}

pub struct Square(pub u32);

impl Shape for Square {
	fn area(&self) -> u32 {
		self.0 * self.0
	}
}

pub struct Circle;

impl Shape for Circle {
	fn area(&self) -> u32 {
		3
	}
}

pub fn boxed(n: u32) -> Box<dyn Shape> {
	if n > LIMIT { Box::new(Circle) } else { Box::new(Square(n)) }
}

pub fn total(shapes: &[Box<dyn Shape>]) -> u32 {
	shapes.iter().map(|s| s.area()).sum()
}

pub fn double(x: u32) -> u32 {
	x * 2
}

pub fn triple(x: u32) -> u32 {
	x * 3
}

pub static OPS: [fn(u32) -> u32; 1] = [triple];

pub fn generic<T: Shape>(t: T) -> u32 {
	t.area() + helper()
}

fn helper() -> u32 {
	1
}

pub fn unused() -> u32 {
	7
}

#[cfg(test)]
mod tests {
	#[test]
	fn squares() {
		assert_eq!(super::Square(2).area(), 4);
	}

	use super::Shape;
}

pub fn where_am_i() -> u32 {
	line!()
}

pub struct Meta {
	pub name: &'static str,
	pub line: Option<u32>,
	pub file: Option<&'static str>,
}

#[macro_export]
macro_rules! inner {
	($name:expr) => {{
		static META: $crate::Meta = $crate::Meta { name: $name, line: Some(line!()), file: Some(file!()) };
		&META
	}};
}

#[macro_export]
macro_rules! outer {
	($name:expr) => {
		$crate::inner!($name)
	};
}

pub fn traced() -> u32 {
	outer!("hello").line.unwrap()
}

pub fn unwrapping(x: Option<u32>) -> u32 {
	x.unwrap() + x.expect("some")
}

pub fn asserting(x: u32) -> u32 {
	assert!(x > 1, "x is {x}");
	x
}
