//! `f64`: a JS number, which is the same IEEE 754 double, so its arithmetic and
//! comparisons are JS's operators, rounded as Rust's are. Where JS differs from std, as
//! in rounding half away from zero, `max` and `min` around NaN, and casts that saturate,
//! the runtime has std's. The functions std leaves to the platform's libm are `Math`'s.

use rustc_middle::mir::{BinOp, UnOp};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::body::Body;
use super::num::{Int, Operand};
use crate::ts::{Expr, Type};

/// The TypeScript type of an `f64`.
pub fn ty() -> Type {
	Type::Rt("f64", Vec::new())
}

/// Whether `ty` is `f64`. The other float types have no JS value yet.
pub fn is(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Float(ty::FloatTy::F64))
}

/// `v` as a literal: the shortest digits that read back as it, as Rust's `Debug` prints them.
/// A NaN other than `f64::NAN` is made from its bits, which keeps its sign and payload.
pub fn lit(v: f64) -> Expr {
	match v {
		_ if v.is_nan() && v.to_bits() != f64::NAN.to_bits() => {
			let u64 = Int {
				bits: 64,
				signed: false,
				pointer: false,
			};
			Expr::rt("f64FromBits", vec![u64.lit(i128::from(v.to_bits()))])
		}
		_ if v.is_nan() => Expr::lit("NaN"),
		f64::INFINITY => Expr::lit("Infinity"),
		f64::NEG_INFINITY => Expr::lit("-Infinity"),
		_ => Expr::lit(format!("{v:?}")),
	}
}

/// `l op r` of two `f64`s, which JS's operators round as Rust's do.
pub fn binary<'tcx>(b: &mut Body<'_, '_, 'tcx>, op: BinOp, l: Expr, r: Expr, span: Span) -> Expr {
	let js = match op {
		BinOp::Add => "+",
		BinOp::Sub => "-",
		BinOp::Mul => "*",
		BinOp::Div => "/",
		// Both are C's `fmod`: exact, with the dividend's sign.
		BinOp::Rem => "%",
		BinOp::Eq => "===",
		BinOp::Ne => "!==",
		BinOp::Lt => "<",
		BinOp::Le => "<=",
		BinOp::Gt => ">",
		BinOp::Ge => ">=",
		_ => return b.err(span, format!("`{op:?}` on `f64`")),
	};
	l.binary(js, r)
}

pub fn unary<'tcx>(b: &mut Body<'_, '_, 'tcx>, op: UnOp, value: Expr, span: Span) -> Expr {
	match op {
		UnOp::Neg => Expr::unary("-", value),
		_ => b.err(span, format!("`{op:?}` on `f64`")),
	}
}

/// `value as to` where either side is a float.
pub fn cast<'tcx>(b: &mut Body<'_, '_, 'tcx>, value: Operand, from: Ty<'tcx>, to: Ty<'tcx>, span: Span) -> Expr {
	match (Int::of(from), Int::of(to)) {
		_ if is(from) && is(to) => value.expr,
		// `From<bool>`, which `as` has no form of.
		_ if from.is_bool() && is(to) => Expr::Cond(Box::new(value.expr), Box::new(lit(1.0)), Box::new(lit(0.0))),
		// An integer to the nearest `f64`, rounded once: exact up to 2^53, which narrow ones are.
		(Some(int), None) if is(to) => match value.value {
			Some(c) if int.signed => lit(c as f64),
			Some(c) => lit(c as u128 as f64),
			None if int.big() => int.method("toF64", vec![value.expr]),
			None => Expr::As(Box::new(value.expr), ty()),
		},
		// Truncated, saturated at the type's bounds, and 0 for NaN.
		(None, Some(int)) if is(from) => int.method("fromF64", vec![value.expr]),
		_ => b.err(span, format!("a cast from `{from}` to `{to}`")),
	}
}

/// A call of `f64`'s own `name`, or `None` if it is not mapped.
pub fn method<'tcx>(b: &mut Body<'_, '_, 'tcx>, name: &str, args: Vec<Expr>, span: Span) -> Option<Expr> {
	let math = |f: &str, args: Vec<Expr>| Expr::ident("Math").member(f).call(args);
	let rt = |f: &str, args: Vec<Expr>| Expr::rt_of("f64", f).call(args);
	let one = |args: Vec<Expr>| <[Expr; 1]>::try_from(args).ok().map(|[x]| x);
	Some(match name {
		// Exact in JS as in std.
		"floor" | "ceil" | "trunc" | "abs" | "sqrt" => math(name, args),
		// libm's in Rust, the engine's in JS: each within a few ULPs, the README says how many.
		"exp" | "log2" | "log10" | "cbrt" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh"
		| "tanh" | "asinh" | "acosh" | "atan2" | "hypot" => math(name, args),
		"ln" => math("log", args),
		"exp_m1" => math("expm1", args),
		"ln_1p" => math("log1p", args),
		"exp2" => math("pow", vec![Expr::lit("2"), one(args)?]),
		"log" => {
			let [x, base] = <[Expr; 2]>::try_from(args).ok()?;
			math("log", vec![x]).binary("/", math("log", vec![base]))
		}
		"is_nan" => Expr::ident("Number").member("isNaN").call(args),
		"is_finite" => Expr::ident("Number").member("isFinite").call(args),
		"recip" => Expr::lit("1").binary("/", one(args)?),
		// std multiplies by these constants, rounded once.
		"to_degrees" => one(args)?.binary("*", lit(180.0 / std::f64::consts::PI)),
		"to_radians" => one(args)?.binary("*", lit(std::f64::consts::PI / 180.0)),
		"round" | "round_ties_even" | "fract" | "signum" | "copysign" | "is_infinite" | "is_normal"
		| "is_subnormal" | "is_sign_positive" | "is_sign_negative" | "next_up" | "next_down" | "max" | "min"
		| "midpoint" | "to_bits" | "from_bits" | "to_be_bytes" | "to_le_bytes" | "from_be_bytes" | "from_le_bytes"
		| "total_cmp" | "powi" | "powf" | "sin_cos" | "atanh" | "div_euclid" | "rem_euclid" | "clamp" => {
			rt(&super::camel(name), args)
		}
		// wasm32 is little-endian.
		"to_ne_bytes" => rt("toLeBytes", args),
		"from_ne_bytes" => rt("fromLeBytes", args),
		"mul_add" => b.err(span, "`f64::mul_add`, which rounds once, where JS would round twice"),
		_ => return None,
	})
}
