//! Formatting: `format!` and `write!` with every option, derived and written `Debug` and
//! `Display`, and panics that format their messages.

#![allow(dead_code, reason = "fields only `Debug` reads")]

use std::fmt;
use std::ops::{Range, RangeToInclusive};
use std::rc::Rc;
use std::sync::Arc;

use crate::generics::Wrap;
use crate::shapes::{Point, Shape};

/// Take an integer's big-endian bytes from the front of `data`, zero-padded.
macro_rules! take {
	($t:ty, $data:expr) => {{
		let data: &[u8] = $data;
		let mut buf = [0u8; core::mem::size_of::<$t>()];
		let n = data.len().min(buf.len());
		buf[..n].copy_from_slice(&data[..n]);
		<$t>::from_be_bytes(buf)
	}};
}

/// Every option on one integer: width, fill, alignment, signs, zeros, precision, and
/// each radix and exponent.
macro_rules! specs {
	($t:ty, $data:expr, $out:expr) => {{
		let v = take!($t, $data);
		put(
			$out,
			&format!(
				"{v} {v:?} {v:5}|{v:<5}|{v:^7}|{v:>4}|{v:*^9}|{v:é>6}|{v:05}|{v:<05}|{v:x} {v:X} {v:#x} {v:#X} {v:08x} {v:#010x} {v:>8x}|{v:<#6X}|{v:#?} {v:3?}"
			),
		);
		put(
			$out,
			&format!(
				"{v:+} {v:+06} {v:+#x} {v:-} {v:.3} {v:8.2}|{v:b} {v:#b} {v:#012b} {v:o} {v:#o} {v:+o}|{v:x?} {v:#X?} {v:06x?} {v:+?}"
			),
		);
		put(
			$out,
			&format!("{v:e} {v:E} {v:.0e} {v:.1e} {v:.3e} {v:.45E} {v:+.2e} {v:012.1e} {v:<+9e}| {v:#e} {v:>12.4E}|"),
		);
		put($out, &v.to_string());
	}};
}

pub fn put(out: &mut Vec<u8>, s: &str) {
	out.extend_from_slice(s.as_bytes());
	out.push(0xff);
}

/// Integers of every width, with each spec.
pub fn ints(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	match kind % 12 {
		0 => specs!(u8, data, out),
		1 => specs!(u16, data, out),
		2 => specs!(u32, data, out),
		3 => specs!(u64, data, out),
		4 => specs!(u128, data, out),
		5 => specs!(usize, data, out),
		6 => specs!(i8, data, out),
		7 => specs!(i16, data, out),
		8 => specs!(i32, data, out),
		9 => specs!(i64, data, out),
		10 => specs!(i128, data, out),
		_ => specs!(isize, data, out),
	}
}

/// Strings that `Debug` escapes, or that pad by chars rather than bytes.
const STRINGS: [&str; 14] = [
	"",
	"a",
	"héllo",
	"日本語",
	"tab\tnew\nline\r",
	"quote\"back\\slash'",
	"\0\x01\x1f\x7f",
	"\u{80}\u{9f}\u{a0}\u{ad}",
	"e\u{301}",
	"\u{200d}\u{feff}\u{115f}\u{2028}\u{3000}",
	// Unicode 18, newer than an engine may know, assigns U+3D000 but not U+3FC40.
	"\u{e000}\u{378}\u{10fffd}\u{3d000}\u{3fc40}",
	"😀 \u{1f1e6}",
	"\u{ff9e}\u{ff9f}\u{ff9d}",
	"{}",
];

/// Strings padded and escaped, and bytes that may not be UTF-8.
pub fn strings(data: &[u8], out: &mut Vec<u8>) {
	for &b in &data[..data.len().min(3)] {
		let s = STRINGS[b as usize % STRINGS.len()];
		put(
			out,
			&format!("{s}|{s:8}|{s:>8}|{s:^8}|{s:-<3}|{s:?}|{s:12?}|{s:#?}|{{{s}}}"),
		);
		// A precision cuts by chars, then the width pads; `Debug` of a string takes neither.
		put(
			out,
			&format!("{s:.2}|{s:8.3}|{s:>6.1}|{s:_^7.2}|{s:.0}|{s:.20}|{s:.2?}|{s:10.2?}|{s:+}"),
		);
	}
	let c = char::from_u32(u32::from(data.first().copied().unwrap_or(0)) * 0x101).unwrap_or('?');
	put(out, &format!("{c}|{c:.0}|{c:3.1}|{c:.0?}|{c:>4?}"));
	match String::from_utf8(data.to_vec()) {
		Ok(s) => put(out, &format!("{s:?} {s:>4} {s:.1}")),
		Err(e) => put(out, &format!("{e} {e:?} {e:#?} {e:>80} {e:.4} {e:x?} {e:#X?}")),
	}
	let flag = matches!(data.first(), Some(b) if b % 2 == 0);
	put(
		out,
		&format!(
			"{flag} {flag:?} {flag:>6} {flag:.2} {flag:>4.1?} {:?} {:#?} {:.1?}",
			(),
			(),
			()
		),
	);
	put(out, &flag.to_string());
	match u8::try_from(data.len() as i32 - 3) {
		Ok(v) => put(out, &format!("{v}")),
		Err(e) => put(out, &format!("{e} {e:?} {e:>40}| {e:.6}|{e:10.3}|")),
	}
}

#[derive(Debug)]
pub struct Unit;

#[derive(Debug, PartialEq)]
pub struct Braced {}

#[derive(Debug, PartialEq)]
pub struct Bare();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair(pub u8, pub i64);

/// Past five fields, derived `Debug` takes the fields as slices.
#[derive(Debug)]
pub struct Wide {
	a: u8,
	b: u16,
	c: u32,
	d: u64,
	e: i8,
	f: bool,
}

#[derive(Debug)]
pub struct Six(u8, u8, u8, u8, u8, u8);

#[derive(Debug)]
pub enum Kind {
	Empty,
	Braced {},
	Bare(),
	One(u8),
	Many { count: u32, items: Vec<u8> },
}

#[derive(Debug)]
pub struct Nested {
	name: String,
	point: Point,
	shape: Shape,
	tags: Vec<u8>,
	pairs: Vec<Pair>,
	maybe: Option<Pair>,
	none: Option<u8>,
	result: Result<u8, String>,
	list: [i16; 2],
	tuple: (u8, bool, &'static str),
	single: (u8,),
	empty: Vec<u8>,
	unit: (),
	boxed: Box<Kind>,
}

fn nested(data: &[u8]) -> Nested {
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	Nested {
		name: STRINGS[at(0) as usize % STRINGS.len()].to_string(),
		point: Point {
			x: at(1) as i32 - 128,
			y: -7,
		},
		shape: match at(2) % 3 {
			0 => Shape::Dot,
			1 => Shape::Line(Point { x: 1, y: 2 }, Point { x: -3, y: 4 }),
			_ => Shape::Rect {
				corner: Point { x: 0, y: 0 },
				width: at(3) as u16,
				height: 9,
			},
		},
		tags: data[..data.len().min(4)].to_vec(),
		pairs: [Pair(at(4), -1), Pair(2, i64::MIN)].to_vec(),
		maybe: if at(5) % 2 == 0 { Some(Pair(at(5), 5)) } else { None },
		none: None,
		result: if at(6) % 2 == 0 {
			Ok(at(6))
		} else {
			Err("no".to_string())
		},
		list: [at(7) as i16 - 100, i16::MAX],
		tuple: (at(8), true, "t"),
		single: (at(9),),
		empty: Vec::new(),
		unit: (),
		boxed: Box::new(match at(10) % 3 {
			0 => Kind::Empty,
			1 => Kind::One(at(11)),
			_ => Kind::Many {
				count: 2,
				items: [at(11), at(12)].to_vec(),
			},
		}),
	}
}

/// Derived `Debug` of every shape, compact and pretty, with options passed to fields.
pub fn derived(data: &[u8], out: &mut Vec<u8>) {
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	let value = nested(data);
	put(out, &format!("{value:?}"));
	put(out, &format!("{value:#?}"));
	// Each field takes the options: hex for integers, and the precision for strings.
	put(out, &format!("{value:x?}"));
	put(out, &format!("{value:#X?}"));
	put(out, &format!("{value:+.1?}"));
	let wide = Wide {
		a: at(0),
		b: 300,
		c: 70_000,
		d: u64::MAX,
		e: -1,
		f: at(1) > 10,
	};
	let six = Six(at(0), 1, 2, 3, 4, at(1));
	put(out, &format!("{wide:?} {six:?} {:?} {:#?}", Unit, Unit));
	put(out, &format!("{wide:#?} {six:#?}"));
	let point = Point { x: at(2) as i32, y: -1 };
	put(
		out,
		&format!("{point:5?}|{point:<4?}|{point:#4?}|{:>3?}", Pair(at(3), -9)),
	);
	let kinds = [
		Kind::Empty,
		Kind::One(at(4)),
		Kind::Braced {},
		Kind::Bare(),
		Kind::Many {
			count: 0,
			items: Vec::new(),
		},
	];
	put(out, &format!("{kinds:?} {:?}", &&kinds[1]));
	put(out, &format!("{kinds:#?}"));
	let bytes: &[u8] = data;
	let words = [at(5) as u16, 0xffff];
	put(
		out,
		&format!("{bytes:?} {words:?} {:?} {:?}", data.first(), data.get(100)),
	);
	put(
		out,
		&format!("{:?} {:#?} {:?} {:#?}", Braced {}, Braced {}, Bare(), Bare()),
	);
	out.extend_from_slice(&[(Braced {} == Braced {}) as u8, (Bare() == Bare()) as u8]);
}

/// `Display` written with `write!`, which ignores the outer options, unless `#`.
pub struct Hex(pub u32);

/// Through to the field, which takes the options.
impl fmt::Binary for Hex {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Binary::fmt(&self.0, f)
	}
}

impl fmt::Octal for Hex {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:o}", self.0)
	}
}

impl fmt::LowerExp for Hex {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::LowerExp::fmt(&self.0, f)
	}
}

impl fmt::UpperExp for Hex {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::UpperExp::fmt(&self.0, f)
	}
}

impl fmt::Display for Hex {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match f.alternate() {
			true => write!(f, "{:#x}", self.0),
			false => write!(f, "{:x}", self.0),
		}
	}
}

impl fmt::Debug for Hex {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&self.0, f)
	}
}

/// `Display` through `pad`, which keeps the options. Only `added` prints its `Debug`.
#[derive(Debug)]
pub struct Padded(&'static str);

impl fmt::Display for Padded {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match f.width() {
			Some(width) if width > 20 => f.write_str("wide"),
			_ => f.pad(self.0),
		}
	}
}

/// `Display` that fails after writing, for odd values.
pub struct Fails(pub u8);

impl fmt::Display for Fails {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.0 % 2 == 0 {
			return f.write_str("fine");
		}
		write!(f, "half {}", self.0)?;
		Err(fmt::Error)
	}
}

/// The builders, written out.
pub struct Manual {
	id: u8,
	list: Vec<u16>,
}

impl fmt::Debug for Manual {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Manual")
			.field("id", &self.id)
			.field("list", &self.list)
			.finish_non_exhaustive()
	}
}

pub struct Seq(Vec<Pair>);

impl fmt::Debug for Seq {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_list().entries(self.0.iter()).finish()
	}
}

pub struct Tup(u8, Option<u8>);

impl fmt::Debug for Tup {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.1 {
			Some(_) => f.debug_tuple("Tup").field(&self.0).field(&self.1).finish(),
			None => f.debug_tuple("Tup").field(&self.0).finish_non_exhaustive(),
		}
	}
}

/// `v`, logged, so an argument evaluated more than once shows.
fn note(log: &mut Vec<u8>, v: u8) -> u8 {
	log.push(v);
	v
}

/// `Display` through `write!` of arguments with effects.
pub struct Noted(pub u8);

impl fmt::Display for Noted {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut log = Vec::new();
		write!(
			f,
			"{} {:?} {log:?}",
			note(&mut log, self.0),
			note(&mut log, 1) == self.0
		)
	}
}

/// Arguments that read `log` before and after one changes it, through a reference.
fn logged(log: &mut Vec<u8>, data: &[u8]) -> String {
	format!(
		"{} {} {} {}",
		log.len(),
		note(log, data[0]),
		data[log.len() % data.len()],
		log.len()
	)
}

/// Written `Display` and `Debug`, and a `Display` that fails inside `format!`.
pub fn written(data: &[u8], out: &mut Vec<u8>) {
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	let hex = Hex(u32::from(at(0)) * 0x1001);
	put(out, &format!("{hex}|{hex:>12}|{hex:#}|{hex:?}|{hex:>6?}|{hex:#?}"));
	// `Debug` through `Display` is decimal under `{:x?}` too.
	put(
		out,
		&format!("{hex:x?}|{hex:#b}|{hex:>8o}|{hex:.2e}|{hex:E}|{hex:+.1?}"),
	);
	put(out, &hex.to_string());
	let padded = Padded("pad");
	put(out, &format!("{padded}|{padded:>6}|{padded:_^9}|{padded:30}|"));
	let manual = Manual {
		id: at(1),
		list: [1, u16::from(at(2))].to_vec(),
	};
	let empty = Manual {
		id: 0,
		list: Vec::new(),
	};
	put(out, &format!("{manual:?} {manual:#?} {empty:?}"));
	let seq = Seq([Pair(at(3), 0), Pair(1, -1)].to_vec());
	let none = Seq(Vec::new());
	put(out, &format!("{seq:?} {seq:#?} {none:?} {none:#?}"));
	let tup = Tup(at(4), if at(5) % 2 == 0 { Some(at(5)) } else { None });
	put(out, &format!("{tup:?} {tup:#?}"));
	// The first argument is evaluated before the second panics.
	let fine = Fails(at(6) & !1);
	put(out, &format!("{fine} {fine:>9}"));
	put(
		out,
		&format!("{1} {0} {0:?} {name} {name:>5}", at(7), at(8), name = "named"),
	);
	// Each argument is evaluated once, in order, however many placeholders read it.
	let mut log = Vec::new();
	let first = format!(
		"{1} {0} {0:?} {2} {}",
		note(&mut log, at(9)),
		note(&mut log, at(10)) == 3,
		log.len()
	);
	put(out, &first);
	put(out, &format!("{}|{:>6}", Noted(at(11)), Noted(2)));
	if !data.is_empty() {
		put(out, &logged(&mut log, data));
	}
	// One argument, read by its one placeholder, is evaluated once, where `format!` is.
	let once = format!("{:?}", note(&mut log, at(13)));
	put(out, &format!("{} {}", once, log.len()));
	out.extend_from_slice(&log);
	if at(12) > 250 {
		panic!("{} {}", note(&mut log, at(12)), log.len());
	}
}

/// Widths and precisions from arguments, by position and by name, and one past
/// `u16::MAX`, which panics.
pub fn counts(data: &[u8], out: &mut Vec<u8>) {
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	let (w, p) = (usize::from(at(0) % 24), usize::from(at(1) % 12));
	let v = take!(i64, &data[data.len().min(2)..]);
	let s = STRINGS[usize::from(at(3)) % STRINGS.len()];
	put(
		out,
		&format!(
			"{v:w$}|{v:<w$e}|{v:+0w$.p$e}|{s:.p$}|{s:^w$.p$}|{:.*}|{2:>0$}|{1:2$.0$}|{s:>p$?}",
			p, s, w
		),
	);
	// Each argument is evaluated once, before any is written, even a count.
	let mut log = Vec::new();
	put(
		out,
		&format!("{:>1$}|{:.*}", note(&mut log, at(4)), usize::from(note(&mut log, 6)), s),
	);
	out.extend_from_slice(&log);
	let big = usize::from(u16::MAX) + usize::from(at(5) % 2);
	put(out, &format!("{s:.big$}|"));
}

/// `{:e}` at each rounding: below, above, and ties to even, which may carry a digit.
pub fn rounding(out: &mut Vec<u8>) {
	for v in [0u32, 5, 15, 25, 125, 135, 994, 995, 999, 1000, 1_250_000, 4_294_967_295] {
		put(out, &format!("{v:.0e} {v:.1e} {v:.2e} {v:e}"));
	}
	put(out, &format!("{:.0e} {:e} {:.3E}", u128::MAX, i128::MIN, i64::MIN));
}

/// Panics, each with the message native Rust formats.
pub fn panics(kind: u8, rest: &[u8], out: &mut Vec<u8>) {
	let n = rest.first().copied().unwrap_or(0);
	match kind % 20 {
		0 => panic!("bad {n}: {:?}", rest),
		1 => unreachable!("never {n:#x} {}", Hex(n.into())),
		2 => assert!(n > 200, "small {} {:?}", n, Pair(n, 0)),
		3 => assert!(n > 200),
		4 => assert_eq!(n, 7),
		5 => assert_ne!(Pair(n, 1), Pair(n, 1), "same {}", n),
		6 => assert_eq!(n, 9, "with {}", "context"),
		7 => {
			let r: Result<u8, Kind> = match n % 2 {
				0 => Err(Kind::One(n)),
				_ => Err(Kind::Many {
					count: 1,
					items: [n].to_vec(),
				}),
			};
			put(out, &r.unwrap().to_string());
		}
		8 => {
			let r: Result<u8, Pair> = if n > 128 { Ok(n) } else { Err(Pair(n, -2)) };
			put(out, &r.expect("expected a byte").to_string());
		}
		9 => {
			let r: Result<Point, u8> = Ok(Point { x: n as i32, y: 0 });
			put(out, &r.unwrap_err().to_string());
		}
		10 => {
			let missing = rest.get(200).copied();
			put(out, &missing.expect(&format!("missing {n}")).to_string());
		}
		11 => put(out, &format!("{}", Fails(n))),
		12 => put(out, &Fails(n).to_string()),
		13 => panic!("{} then {}", Hex(n.into()), Fails(n)),
		14 => {
			let (start, end) = ((n % 8) as usize, (n / 32) as usize);
			put(out, &format!("{:?} {:?}", &rest[start..], &rest[start..end]));
		}
		15 => {
			let mut buf = [0u8; 3];
			buf.copy_from_slice(&rest[..rest.len().min(4)]);
			put(out, &format!("{buf:?}"));
		}
		16 => {
			let r: Result<u8, std::string::FromUtf8Error> = match String::from_utf8(rest.to_vec()) {
				Ok(s) => Ok(s.len() as u8),
				Err(e) => Err(e),
			};
			put(out, &r.unwrap().to_string());
		}
		17 => {
			let mut buf = [1u8, 2, 3, 4];
			let dest = rest.get(1).copied().unwrap_or(0) as usize % 4;
			buf.copy_within((n % 6) as usize..(n / 6 % 6) as usize, dest);
			put(out, &format!("{buf:?}"));
		}
		18 => put(out, &rest[(n % 8) as usize].to_string()),
		_ => {
			let r: Result<u8, u8> = if n % 2 == 0 { Err(n) } else { Ok(n) };
			put(out, &format!("{}", r.unwrap_err()));
		}
	}
}

/// `Debug` that pads its text, so a width and precision passed to it show, and marks `#`.
#[derive(Clone, Copy)]
pub struct Clip(&'static str);

impl fmt::Debug for Clip {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if f.alternate() {
			f.write_str("#")?;
		}
		f.pad(self.0)
	}
}

#[derive(Debug)]
pub struct Shared {
	arc: Arc<Pair>,
	span: Range<u16>,
	upto: RangeToInclusive<u8>,
}

/// std's ranges' `Debug`, which passes its options to each bound, and an `Arc`'s `Debug`
/// and `Display`, which pass them to what it holds.
pub fn wrapped(data: &[u8], out: &mut Vec<u8>) {
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	let (a, b) = (i32::from(at(0)) - 128, i32::from(at(1)) - 100);
	let w = usize::from(at(2) % 10);
	put(
		out,
		&format!("{:?} {:?} {:?} {:?} {:?} {:?}", a..b, a..=b, a.., ..b, ..=b, ..),
	);
	put(
		out,
		&format!(
			"{:5?}|{:<4?}|{:+?}|{:05?}|{:x?}|{:#X?}|{:w$?}|{:^w$?}|{:>3?}|{:#?}",
			a..b,
			a..=b,
			a..,
			..b,
			..=b,
			a..b,
			a..=b,
			..=b,
			..,
			a..b
		),
	);
	let lo = Clip(STRINGS[usize::from(at(3)) % STRINGS.len()]);
	let hi = Clip(STRINGS[usize::from(at(4)) % STRINGS.len()]);
	put(
		out,
		&format!(
			"{:?}|{:8.2?}|{:#?}|{:_>w$.1?}|{:#4?}",
			lo..hi,
			lo..=hi,
			..hi,
			lo..,
			..=lo
		),
	);
	let (x, y) = (u32::from(at(5)) * 0x101, u32::from(at(6)));
	put(
		out,
		&format!("{:?}|{:>9?}|{:#?}", Hex(x)..Hex(y), Hex(x)..=Hex(y), ..Hex(y)),
	);
	let pairs = Pair(at(7), -1)..Pair(1, i64::MIN);
	put(out, &format!("{pairs:?}|{pairs:#?}|{pairs:x?}|{pairs:+?}"));
	// Iteration leaves an inclusive range in the state its `Debug` prints.
	let mut drained = (at(8) % 4)..=(at(9) % 4);
	while drained.next().is_some() {}
	let mut top = at(8).max(250)..=255;
	while top.next().is_some() {}
	let mut back = 0..=(at(9) % 3);
	while back.next_back().is_some() {}
	let mut far = at(8)..=at(9);
	far.nth(usize::from(at(10) % 8));
	put(out, &format!("{drained:?} {top:?} {back:?} {far:?} {:?}", 0..=0));

	let arc = Arc::new(Hex(x));
	put(out, &format!("{arc}|{arc:>12}|{arc:#}|{arc:?}|{arc:>6?}|{arc:#?}"));
	put(out, &arc.to_string());
	let nested = Arc::new(Arc::new(a));
	put(
		out,
		&format!("{nested}|{nested:+05}|{nested:?}|{nested:x?}|{nested:#X?}|{nested:<6?}|{nested:^+w$}"),
	);
	let rc = Rc::new(STRINGS[usize::from(at(3)) % STRINGS.len()].to_string());
	put(out, &format!("{rc}|{rc:>8.2}|{rc:?}"));
	let list = Arc::new([Pair(at(7), 2), Pair(3, -4)].to_vec());
	put(out, &format!("{list:?} {list:#?}"));
	let held = Arc::new(a)..Arc::new(b);
	put(out, &format!("{held:?} {held:+04?} {:?}", Arc::new(lo)..Arc::new(hi)));
	let shared = Shared {
		arc: Arc::new(Pair(at(7), 5)),
		span: u16::from(at(8))..300,
		upto: ..=at(9),
	};
	put(out, &format!("{shared:?} {shared:#?}"));
	let t: Arc<str> = Arc::from(STRINGS[usize::from(at(3)) % STRINGS.len()]);
	put(out, &format!("{t} {t:?} {t:>9.2}"));
}

/// An error only a TypeScript caller prints, as moq's callers print its errors: nothing
/// translated formats it, so its `Display` and `Debug` come along as `printable`.
#[derive(Debug)]
pub enum Fault {
	Short,
	Code(u16),
	TooLarge { size: usize, max: usize },
	At(Pair),
}

impl fmt::Display for Fault {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Short => f.pad("short buffer"),
			Self::Code(code) => write!(f, "code={code:#06x}"),
			Self::TooLarge { size, max } => write!(f, "{size} bytes exceeds the {max} byte limit"),
			Self::At(pair) => write!(f, "at {pair:?}"),
		}
	}
}

impl Fault {
	/// The fault `data` picks.
	pub fn new(data: &[u8]) -> Self {
		let at = |i: usize| data.get(i).copied().unwrap_or(0);
		match at(0) % 4 {
			0 => Self::Short,
			1 => Self::Code(u16::from_be_bytes([at(1), at(2)])),
			2 => Self::TooLarge {
				size: usize::from(at(1)) << 24 | usize::from(at(2)),
				max: usize::from(at(3)),
			},
			_ => Self::At(Pair(at(1), i64::from(at(2) as i8) << 40)),
		}
	}
}

/// A [`Fault`] as native Rust prints it, which the translated side prints from TypeScript.
pub fn printed(data: &[u8]) -> Vec<u8> {
	let fault = Fault::new(data);
	format!("{fault}|{fault:>16}|{fault:?}|{fault:#?}").into_bytes()
}

/// A root only the `stable_names` test adds, reaching another instance of what `run`
/// reaches one of: a generic method's, a closure's, and an impl's of another trait.
pub fn added(out: &mut Vec<u8>) {
	out.push(Wrap::new(1u8).into_inner());
	let mapped = Wrap::new(2u8).map(|b| Pair(b, 0));
	put(out, &format!("{mapped:?} {:?}", Padded("added")));
}

/// A writer of the crate's own that fails a `write_str` past `cap` bytes, and counts each.
pub struct Capped {
	text: String,
	calls: u8,
	cap: usize,
}

impl fmt::Write for Capped {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.calls += 1;
		if self.text.len() + s.len() > self.cap {
			return Err(fmt::Error);
		}
		for c in s.chars() {
			self.text.push(c);
		}
		Ok(())
	}
}

/// A writer of the crate's own that writes each `char` twice through its own `write_char`.
pub struct Twice(String);

impl fmt::Write for Twice {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		for c in s.chars() {
			self.0.push(c);
		}
		Ok(())
	}

	fn write_char(&mut self, c: char) -> fmt::Result {
		self.0.push(c);
		self.0.push(c);
		Ok(())
	}
}

/// Literals, a `char` of each width, and empty pieces, each its own call, stopping at the
/// first error.
fn literals<W: fmt::Write>(mut w: W, c: char) -> fmt::Result {
	w.write_str("ab")?;
	write!(w, "")?;
	write!(w, "{}", "")?;
	w.write_char(c)?;
	write!(w, "c{}d", "x")?;
	w.write_char('😀')?;
	writeln!(w)?;
	write!(w, "{{}}")?;
	w.write_str("")?;
	writeln!(w, "end")
}

/// Placeholders with options, through a writer std's impl forwards to.
fn placed<W: fmt::Write>(mut w: W, n: u8) -> fmt::Result {
	write!(w, "n={n:03}|{n:>4x}|{:?}|{:<3}|{:+}", "q\"é", 'ß', i16::from(n) - 100)?;
	w.write_char('é')?;
	writeln!(w, "{n:#b}")
}

/// `Display` through a `Formatter` as a writer, which takes no options from the placeholder.
pub struct Placed(u8);

impl fmt::Display for Placed {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		placed(&mut *f, self.0)?;
		literals(f, 'é')
	}
}

/// Each result as a byte: 1 for `Ok`.
fn result(out: &mut Vec<u8>, r: fmt::Result) {
	out.push(r.is_ok() as u8);
}

/// `fmt::Write` for a `String`, through `&mut W`, and for the crate's own writers.
pub fn writes(data: &[u8], out: &mut Vec<u8>) {
	use std::fmt::Write as _;
	let at = |i: usize| data.get(i).copied().unwrap_or(0);
	let c = char::from_u32(u32::from(at(0)) * 0x101).unwrap_or('?');
	let mut s = String::new();
	result(out, write!(s, "a{}b{:>5}|{:?}|{:x}", at(1), c, "q\"é", at(2)));
	result(out, writeln!(s));
	result(out, s.write_char(c));
	result(out, s.write_str("tail"));
	// An error from a placeholder stops after what it wrote, before the next piece.
	result(out, write!(s, "x{}y{}", Fails(at(3)), at(4)));
	result(out, literals(&mut s, c));
	result(out, placed(&mut s, at(5)));
	let mut inner = &mut s;
	result(out, literals(&mut inner, 'ß'));
	result(out, placed(&mut &mut inner, at(6)));
	put(out, &s);
	put(out, &format!("{}|{:>30}", Placed(at(7)), Placed(at(8))));
	let mut capped = Capped {
		text: String::new(),
		calls: 0,
		cap: usize::from(at(9) % 40),
	};
	result(out, literals(&mut capped, c));
	result(out, capped.write_char(c));
	result(out, write!(capped, "z"));
	result(out, literals(&mut &mut capped, 'é'));
	out.push(capped.calls);
	put(out, &capped.text);
	let mut twice = Twice(String::new());
	result(out, literals(&mut twice, c));
	result(out, twice.write_char('ß'));
	put(out, &twice.0);
}
