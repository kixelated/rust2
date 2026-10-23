//! Integers: literals, arithmetic, and casts with Rust's semantics.
//!
//! Integers up to 32 bits are JS numbers, in which their sums, differences, and
//! products are exact or provably out of range, so each is checked as computed. Wider
//! integers are bigints. Every operation that can overflow is checked, as a debug build
//! checks it: none wraps silently.

use rustc_middle::mir::{BinOp, UnOp};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::body::Body;
use crate::ts::Expr;

/// An integer type, by width and signedness. `usize` and `isize` are 32 bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Int {
	pub bits: u32,
	pub signed: bool,
	/// `usize` or `isize`, which are named so in the runtime.
	pub pointer: bool,
}

impl Int {
	pub fn of(ty: Ty<'_>) -> Option<Self> {
		let (bits, signed, pointer) = match ty.kind() {
			ty::Int(int) => match int {
				ty::IntTy::Isize => (32, true, true),
				int => (int.bit_width().unwrap() as u32, true, false),
			},
			ty::Uint(uint) => match uint {
				ty::UintTy::Usize => (32, false, true),
				uint => (uint.bit_width().unwrap() as u32, false, false),
			},
			_ => return None,
		};
		Some(Self { bits, signed, pointer })
	}

	/// Whether values are bigints rather than numbers.
	pub fn big(self) -> bool {
		self.bits > 32
	}

	pub fn min(self) -> i128 {
		match self.signed {
			true => -(1i128 << (self.bits - 1)),
			false => 0,
		}
	}

	pub fn max(self) -> i128 {
		match (self.signed, self.bits) {
			(true, bits) => (1i128 << (bits - 1)) - 1,
			(false, 128) => i128::MAX,
			(false, bits) => (1i128 << bits) - 1,
		}
	}

	/// The runtime's name for the type: `rt.u32`.
	pub fn rt(self) -> &'static str {
		match (self.signed, self.bits, self.pointer) {
			(false, _, true) => "usize",
			(true, _, true) => "isize",
			(false, 8, _) => "u8",
			(false, 16, _) => "u16",
			(false, 32, _) => "u32",
			(false, 64, _) => "u64",
			(false, _, _) => "u128",
			(true, 8, _) => "i8",
			(true, 16, _) => "i16",
			(true, 32, _) => "i32",
			(true, 64, _) => "i64",
			(true, _, _) => "i128",
		}
	}

	/// `value` as a literal of this type.
	pub fn lit(self, value: i128) -> Expr {
		match self.big() {
			true => Expr::lit(format!("{value}n")),
			false => Expr::lit(value.to_string()),
		}
	}

	/// The runtime method `rt.<type>.<name>`.
	pub fn method(self, name: &str, args: Vec<Expr>) -> Expr {
		Expr::Rt(self.rt()).member(name).call(args)
	}

	/// `value`, panicking if it does not fit this type.
	pub fn check(self, value: Expr) -> Expr {
		Expr::Rt(self.rt()).call(vec![value])
	}

	/// `value`, which is exact, wrapped into this type's range.
	fn wrap(self, value: Expr) -> Expr {
		match (self.big(), self.signed, self.bits) {
			(true, false, bits) => big_int("asUintN", bits, value),
			(true, true, bits) => big_int("asIntN", bits, value),
			(false, false, 32) => value.binary(">>>", Expr::lit("0")),
			(false, true, 32) => value.binary("|", Expr::lit("0")),
			(false, false, bits) => value.binary("&", Expr::lit(format!("0x{:x}", (1u64 << bits) - 1))),
			(false, true, bits) => {
				let shift = Expr::lit((32 - bits).to_string());
				value.binary("<<", shift.clone()).binary(">>", shift)
			}
		}
	}
}

fn big_int(method: &str, bits: u32, value: Expr) -> Expr {
	Expr::ident("BigInt").member(method).call(vec![Expr::lit(bits.to_string()), value])
}

/// An operand, and its value when it is a constant.
pub struct Operand {
	pub expr: Expr,
	pub value: Option<i128>,
}

/// Fold a constant operation on `int`s as Rust would, or `None` if it overflows.
pub fn fold(op: BinOp, a: i128, b: i128, int: Int) -> Option<i128> {
	let value = match op {
		BinOp::Add => a.checked_add(b)?,
		BinOp::Sub => a.checked_sub(b)?,
		BinOp::Mul => a.checked_mul(b)?,
		BinOp::Div if b != 0 => a.checked_div(b)?,
		BinOp::Rem if b != 0 => a.checked_rem(b)?,
		BinOp::BitAnd => a & b,
		BinOp::BitOr => a | b,
		BinOp::BitXor => a ^ b,
		BinOp::Shl if (0..int.bits as i128).contains(&b) => {
			let shifted = a.checked_shl(b as u32)?;
			// Bits shifted out are lost; keep the fold exact by declining then.
			if shifted >> b != a {
				return None;
			}
			shifted
		}
		BinOp::Shr if (0..int.bits as i128).contains(&b) => a >> b,
		_ => return None,
	};
	(int.min()..=int.max()).contains(&value).then_some(value)
}

pub fn binary<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	op: BinOp,
	l: Operand,
	r: Operand,
	lty: Ty<'tcx>,
	rty: Ty<'tcx>,
	span: Span,
) -> Expr {
	let (lty, rty) = (lty.peel_refs(), rty.peel_refs());
	let compare = match op {
		BinOp::Eq => Some("==="),
		BinOp::Ne => Some("!=="),
		BinOp::Lt => Some("<"),
		BinOp::Le => Some("<="),
		BinOp::Gt => Some(">"),
		BinOp::Ge => Some(">="),
		_ => None,
	};
	if let Some(js) = compare {
		return match lty.kind() {
			ty::Bool | ty::Int(_) | ty::Uint(_) | ty::Char => l.expr.binary(js, r.expr),
			_ => b.err(span, format!("comparing `{lty}`")),
		};
	}

	if let ty::Bool = lty.kind() {
		return match op {
			BinOp::BitXor => l.expr.binary("!==", r.expr),
			// Both sides are already evaluated, so short-circuiting changes nothing.
			BinOp::BitAnd if r.expr.is_path() => l.expr.binary("&&", r.expr),
			BinOp::BitOr if r.expr.is_path() => l.expr.binary("||", r.expr),
			BinOp::BitAnd => Expr::rt("and", vec![l.expr, r.expr]),
			BinOp::BitOr => Expr::rt("or", vec![l.expr, r.expr]),
			_ => b.err(span, format!("`{op:?}` on `bool`")),
		};
	}

	let Some(int) = Int::of(lty) else {
		return b.err(span, format!("arithmetic on `{lty}`"));
	};

	// Constants fold, written as the source has them where JS computes the same.
	if let (Some(a), Some(c)) = (l.value, r.value)
		&& let Some(value) = fold(op, a, c, int)
	{
		let exact = match int.big() {
			true => true,
			false => match op {
				BinOp::Add | BinOp::Sub | BinOp::Mul => value.abs() < (1 << 53),
				// JS bitwise operators compute in 32-bit signed integers.
				_ => (i32::MIN as i128..=i32::MAX as i128).contains(&value) && a >= 0 && c >= 0,
			},
		};
		if !exact {
			return int.lit(value);
		}
		let rhs = match (op, int.big()) {
			(BinOp::Shl | BinOp::Shr, true) => Expr::lit(format!("{c}n")),
			_ => r.expr,
		};
		let js = match op {
			BinOp::Add => "+",
			BinOp::Sub => "-",
			BinOp::Mul => "*",
			BinOp::Div | BinOp::Rem => return int.lit(value),
			BinOp::BitAnd => "&",
			BinOp::BitOr => "|",
			BinOp::BitXor => "^",
			BinOp::Shl => "<<",
			BinOp::Shr => ">>",
			_ => return int.lit(value),
		};
		return l.expr.binary(js, rhs);
	}

	match op {
		BinOp::Add => int.check(l.expr.binary("+", r.expr)),
		BinOp::Sub => int.check(l.expr.binary("-", r.expr)),
		BinOp::Mul => int.check(l.expr.binary("*", r.expr)),
		BinOp::Div => int.method("div", vec![l.expr, r.expr]),
		BinOp::Rem => int.method("rem", vec![l.expr, r.expr]),
		BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
			let js = match op {
				BinOp::BitAnd => "&",
				BinOp::BitOr => "|",
				_ => "^",
			};
			let value = l.expr.binary(js, r.expr);
			// Only an unsigned 32-bit result can land outside a JS int32.
			match (int.big(), int.signed, int.bits) {
				(false, false, 32) => value.binary(">>>", Expr::lit("0")),
				_ => value,
			}
		}
		BinOp::Shl | BinOp::Shr => {
			let amount = match (r.value, Int::of(rty)) {
				(Some(c), _) if (0..int.bits as i128).contains(&c) => Some(c),
				_ => None,
			};
			let Some(c) = amount else {
				let name = if op == BinOp::Shl { "shl" } else { "shr" };
				return int.method(name, vec![l.expr, r.expr]);
			};
			match (op, int.big()) {
				(BinOp::Shl, false) => int.wrap(l.expr.binary("<<", Expr::lit(c.to_string()))),
				(BinOp::Shl, true) => int.wrap(l.expr.binary("<<", Expr::lit(format!("{c}n")))),
				(_, false) if !int.signed && int.bits == 32 => l.expr.binary(">>>", Expr::lit(c.to_string())),
				(_, false) => l.expr.binary(">>", Expr::lit(c.to_string())),
				(_, true) => l.expr.binary(">>", Expr::lit(format!("{c}n"))),
			}
		}
		_ => b.err(span, format!("unsupported operator `{op:?}`")),
	}
}

pub fn unary<'tcx>(b: &mut Body<'_, '_, 'tcx>, op: UnOp, value: Expr, ty: Ty<'tcx>, span: Span) -> Expr {
	let ty = ty.peel_refs();
	match (op, ty.kind()) {
		(UnOp::Not, ty::Bool) => value.not(),
		(UnOp::Not, ty::Int(_) | ty::Uint(_)) => {
			let int = Int::of(ty).unwrap();
			let inverted = Expr::unary("~", value);
			match (int.signed, int.big()) {
				(true, _) => inverted,
				(false, _) => int.wrap(inverted),
			}
		}
		(UnOp::Neg, ty::Int(_)) => {
			let int = Int::of(ty).unwrap();
			int.check(Expr::unary("-", value))
		}
		_ => b.err(span, format!("unsupported `{op:?}` on `{ty}`")),
	}
}

/// `value as to`.
pub fn cast<'tcx>(b: &mut Body<'_, '_, 'tcx>, value: Operand, from: Ty<'tcx>, to: Ty<'tcx>, span: Span) -> Expr {
	let Some(target) = Int::of(to) else {
		return b.err(span, format!("a cast to `{to}`"));
	};
	if let ty::Bool = from.kind() {
		let (one, zero) = match target.big() {
			true => ("1n", "0n"),
			false => ("1", "0"),
		};
		return Expr::Cond(Box::new(value.expr), Box::new(Expr::lit(one)), Box::new(Expr::lit(zero)));
	}
	if let ty::Adt(adt, _) = from.kind()
		&& adt.is_enum()
		&& adt.variants().iter().all(|v| v.fields.is_empty())
	{
		// A fieldless enum's tag to its discriminant.
		let tcx = b.t.tcx;
		let table = adt
			.discriminants(tcx)
			.map(|(index, discr)| {
				let value = match discr.ty.is_signed() {
					true => rustc_abi::Size::from_bits(target.bits.max(8)).sign_extend(discr.val),
					false => discr.val as i128,
				};
				(crate::ts::Key::Name(adt.variant(index).name.to_string()), target.lit(value))
			})
			.collect();
		return Expr::Index(Box::new(Expr::Object(table)), Box::new(value.expr));
	}
	let Some(source) = Int::of(from) else {
		return b.err(span, format!("a cast from `{from}`"));
	};
	if let Some(c) = value.value {
		let size = rustc_abi::Size::from_bits(target.bits);
		let bits = size.truncate(c as u128);
		let folded = match target.signed {
			true => size.sign_extend(bits),
			false => bits as i128,
		};
		return target.lit(folded);
	}
	let fits = target.min() <= source.min() && source.max() <= target.max();
	match (source.big(), target.big()) {
		(false, false) if fits => value.expr,
		(false, false) => target.wrap(value.expr),
		(false, true) => {
			let widened = Expr::ident("BigInt").call(vec![value.expr]);
			match fits {
				true => widened,
				false => target.wrap(widened),
			}
		}
		(true, false) => {
			let method = if target.signed { "asIntN" } else { "asUintN" };
			Expr::ident("Number").call(vec![big_int(method, target.bits, value.expr)])
		}
		(true, true) if fits => value.expr,
		(true, true) => target.wrap(value.expr),
	}
}
