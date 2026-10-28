//! `f64`: arithmetic and comparisons, `as` casts to and from every integer width, its
//! methods and constants, sums, places, and `Duration`'s `f64` methods. Operands are
//! bits, so NaNs, infinities, and subnormals come up; `caller.ts` names the values random
//! bits rarely make. A NaN result is written as one NaN, since neither side specifies
//! which NaN an operation makes, and the functions std leaves to libm, kind 7, agree
//! within the ULPs `caller.ts` allows.

use std::f64::consts;
use std::time::Duration;

use crate::fmt::put;

/// An `f64` from the front of `data`, its bits big-endian and zero-padded.
fn take(data: &[u8]) -> (f64, &[u8]) {
	let mut buf = [0u8; 8];
	let n = data.len().min(buf.len());
	buf[..n].copy_from_slice(&data[..n]);
	(f64::from_bits(u64::from_be_bytes(buf)), &data[n..])
}

/// `x`'s bits, any NaN as `f64::NAN`'s.
fn put_f64(out: &mut Vec<u8>, x: f64) {
	let x = if x.is_nan() { f64::NAN } else { x };
	out.extend_from_slice(&x.to_bits().to_be_bytes());
}

fn put_duration(out: &mut Vec<u8>, d: Duration) {
	out.extend_from_slice(&d.as_secs().to_be_bytes());
	out.extend_from_slice(&d.subsec_nanos().to_be_bytes());
}

/// What `try_from_secs_f64` made, or its error printed both ways.
fn put_secs(out: &mut Vec<u8>, d: Result<Duration, std::time::TryFromFloatSecsError>) {
	match d {
		Ok(d) => put_duration(out, d),
		Err(e) => put(out, &format!("{e}|{e:?}|{e:#?}")),
	}
}

/// A struct with an `f64` field, changed in place through a `&mut`.
#[derive(Clone, Copy, PartialEq, Default)]
struct Gauge {
	level: f64,
	peak: Option<f64>,
}

fn raise(level: &mut f64, by: f64) {
	*level += by;
	*level *= 2.0;
}

/// Each float constant std has, in a `const` of the crate's.
const HALF: f64 = 0.5;
/// A NaN whose sign and payload a constant keeps, which only its bits make.
const PAYLOAD: f64 = f64::from_bits(0xfff8_0000_0000_0123);
const CONSTANTS: [f64; 22] = [
	f64::MAX,
	f64::MIN,
	f64::MIN_POSITIVE,
	f64::EPSILON,
	f64::INFINITY,
	f64::NEG_INFINITY,
	f64::NAN,
	-0.0,
	consts::PI,
	consts::TAU,
	consts::E,
	consts::SQRT_2,
	consts::FRAC_1_SQRT_2,
	consts::LN_2,
	consts::LN_10,
	consts::LOG2_E,
	consts::LOG10_E,
	consts::FRAC_PI_2,
	consts::FRAC_1_PI,
	1e-300,
	123456789.125,
	HALF,
];

pub fn floats(kind: u8, data: &[u8], out: &mut Vec<u8>) {
	let (a, rest) = take(data);
	let (b, rest) = take(rest);
	match kind % 10 {
		0 => {
			// Arithmetic, which JS rounds as Rust does, and comparisons, NaN's included.
			for x in [a + b, a - b, a * b, a / b, a % b, -a, a + 1.5, 0.1 * a] {
				put_f64(out, x);
			}
			for bit in [
				a == b,
				a != b,
				a < b,
				a <= b,
				a > b,
				a >= b,
				a == 0.0,
				a.lt(&b),
				(&a).ge(&&b),
			] {
				out.push(bit as u8);
			}
			let mut c = a;
			c += b;
			c -= 0.25;
			c *= b;
			c /= 3.0;
			c %= a;
			put_f64(out, c);
		}
		1 => {
			// To every integer type: truncated, saturated at its bounds, and 0 for NaN.
			out.extend_from_slice(&(a as u8).to_be_bytes());
			out.extend_from_slice(&(a as u16).to_be_bytes());
			out.extend_from_slice(&(a as u32).to_be_bytes());
			out.extend_from_slice(&(a as u64).to_be_bytes());
			out.extend_from_slice(&(a as u128).to_be_bytes());
			out.extend_from_slice(&(a as usize).to_be_bytes());
			out.extend_from_slice(&(a as i8).to_be_bytes());
			out.extend_from_slice(&(a as i16).to_be_bytes());
			out.extend_from_slice(&(a as i32).to_be_bytes());
			out.extend_from_slice(&(a as i64).to_be_bytes());
			out.extend_from_slice(&(a as i128).to_be_bytes());
			out.extend_from_slice(&(a as isize).to_be_bytes());
		}
		2 => {
			// From every integer type, rounded once to the nearest, ties to even.
			let mut wide = [0u8; 16];
			let n = data.len().min(16);
			wide[..n].copy_from_slice(&data[..n]);
			let v = u128::from_be_bytes(wide);
			for x in [
				v as u8 as f64,
				v as u16 as f64,
				v as u32 as f64,
				v as u64 as f64,
				v as f64,
				v as usize as f64,
				v as i8 as f64,
				v as i16 as f64,
				v as i32 as f64,
				v as i64 as f64,
				v as i128 as f64,
				v as isize as f64,
				f64::from(v as u32),
				f64::from(v as i16),
				f64::from(v & 1 == 1),
				(v as u8).into(),
				u64::MAX as f64,
				i64::MIN as f64,
				u128::MAX as f64,
			] {
				put_f64(out, x);
			}
			// And back, where the round trip saturates.
			out.extend_from_slice(&((v as u64) as f64 as u64).to_be_bytes());
			out.extend_from_slice(&((v as i128) as f64 as i128).to_be_bytes());
		}
		3 => {
			// Rounding, which JS's `Math.round` does half up rather than away from zero.
			for x in [
				a.floor(),
				a.ceil(),
				a.trunc(),
				a.round(),
				a.round_ties_even(),
				a.fract(),
				a.abs(),
				a.signum(),
				a.sqrt(),
				a.recip(),
				a.to_degrees(),
				a.to_radians(),
			] {
				put_f64(out, x);
			}
		}
		4 => {
			// The sign bit and the class, and the doubles next to `a`.
			let bits = [
				a.is_nan(),
				a.is_finite(),
				a.is_infinite(),
				a.is_normal(),
				a.is_subnormal(),
				a.is_sign_positive(),
				a.is_sign_negative(),
			];
			for bit in bits {
				out.push(bit as u8);
			}
			for x in [a.copysign(b), 1.5f64.copysign(a), a.next_up(), a.next_down()] {
				put_f64(out, x);
			}
		}
		5 => {
			// Two operands. std leaves which zero `max` and `min` pick between 0.0 and -0.0.
			let zeros = a == 0.0 && b == 0.0;
			for x in [a.max(b), a.min(b)] {
				put_f64(out, if zeros { 0.0 } else { x });
			}
			for x in [a.midpoint(b), a.div_euclid(b), a.rem_euclid(b)] {
				put_f64(out, x);
			}
			out.push(a.total_cmp(&b) as i8 as u8);
			out.push(a.partial_cmp(&b).map_or(9, |o| o as i8 as u8));
		}
		6 => {
			// Bits and bytes. A NaN's are the engine's, so only a number's round trip.
			if !a.is_nan() {
				out.extend_from_slice(&a.to_bits().to_be_bytes());
				out.extend_from_slice(&a.to_be_bytes());
				out.extend_from_slice(&a.to_le_bytes());
				out.extend_from_slice(&a.to_ne_bytes());
				let be = a.to_be_bytes();
				let le = a.to_le_bytes();
				for x in [f64::from_be_bytes(be), f64::from_le_bytes(le), f64::from_ne_bytes(le)] {
					put_f64(out, x);
				}
			}
			put_f64(out, f64::from_bits(a.to_bits() ^ 1));
		}
		7 => {
			// What std leaves to libm, within the ULPs `caller.ts` allows.
			let n = (b.to_bits() as i32) % 64;
			let (s, c) = a.sin_cos();
			for x in [
				a.sin(),
				a.cos(),
				a.tan(),
				a.asin(),
				a.acos(),
				a.atan(),
				a.atan2(b),
				a.sinh(),
				a.cosh(),
				a.tanh(),
				a.asinh(),
				a.acosh(),
				a.atanh(),
				a.exp(),
				a.exp2(),
				a.exp_m1(),
				a.ln(),
				a.log(b),
				a.log2(),
				a.log10(),
				a.ln_1p(),
				a.powf(b),
				a.powi(n),
				a.cbrt(),
				a.hypot(b),
				s,
				c,
			] {
				put_f64(out, x);
			}
		}
		8 => {
			// `Duration`'s `f64` methods, through std's integer algorithm, with its errors and panics.
			let d = Duration::new(rest.first().map_or(0, |&s| u64::from(s)) << 20, 123_456_789);
			put_f64(out, d.as_secs_f64());
			put_f64(out, d.div_duration_f64(Duration::from_millis(1500)));
			put_f64(out, Duration::ZERO.div_duration_f64(Duration::ZERO));
			put_secs(out, Duration::try_from_secs_f64(a));
			put_secs(out, Duration::try_from_secs_f64(a.abs()));
			put_secs(out, Duration::try_from_secs_f64(a.abs() * 1e-9));
			match kind / 10 % 4 {
				0 => put_duration(out, Duration::from_secs_f64(a)),
				1 => put_duration(out, d.mul_f64(a)),
				2 => put_duration(out, d.div_f64(a)),
				_ => put_duration(out, Duration::MAX.mul_f64(b)),
			}
		}
		_ => {
			// Constants, sums and products, `Default`, places, and fields.
			for x in CONSTANTS {
				put_f64(out, x);
			}
			out.extend_from_slice(&PAYLOAD.to_bits().to_be_bytes());
			out.push(PAYLOAD.is_sign_negative() as u8);
			let v = [a, b, 0.5];
			put_f64(out, v.iter().sum());
			put_f64(out, v.into_iter().product());
			put_f64(out, [-0.0f64; 0].iter().sum());
			put_f64(out, [-0.0f64, -0.0].into_iter().sum::<f64>());
			put_f64(out, f64::default());
			let mut g = Gauge {
				level: a,
				peak: Some(b),
			};
			raise(&mut g.level, b);
			if let Some(peak) = &mut g.peak {
				*peak = peak.max(g.level);
			}
			put_f64(out, g.level);
			put_f64(out, g.peak.unwrap_or(-1.0));
			out.push((g == Gauge::default()) as u8);
			out.push((g.peak == Some(b)) as u8);
			let mut local = a;
			raise(&mut local, 1.0);
			put_f64(out, local);
		}
	}
}
