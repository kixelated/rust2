pub fn apply(f: fn(u32) -> u32, x: u32) -> u32 {
	f(x)
}

pub fn via_pointer(x: u32) -> u32 {
	apply(a::double, x)
}

pub fn via_static(x: u32) -> u32 {
	a::OPS[0](x)
}

pub struct Tri;

impl a::Shape for Tri {
	fn area(&self) -> u32 {
		a::LIMIT
	}
}
