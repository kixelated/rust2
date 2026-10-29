//! Every integer type's arithmetic, one operation per case so a panic hides nothing.

/// Take an integer's big-endian bytes from the front of `data`, zero-padded.
macro_rules! take {
	($t:ty, $data:expr) => {{
		let data: &[u8] = $data;
		let mut buf = [0u8; core::mem::size_of::<$t>()];
		let n = data.len().min(buf.len());
		buf[..n].copy_from_slice(&data[..n]);
		(<$t>::from_be_bytes(buf), &data[n..])
	}};
}

/// `a.div_ceil(b)` of an unsigned type, which a signed one lacks.
macro_rules! div_ceil {
	(unsigned, $t:ty, $a:expr, $b:expr) => {
		Some($a.div_ceil($b))
	};
	(signed, $t:ty, $a:expr, $b:expr) => {
		None::<$t>
	};
}

/// `a.is_multiple_of(b)` of an unsigned type, which a signed one lacks.
macro_rules! is_multiple_of {
	(unsigned, $a:expr, $b:expr) => {
		Some($a.is_multiple_of($b))
	};
	(signed, $a:expr, $b:expr) => {
		None::<bool>
	};
}

macro_rules! ops {
	($name:ident, $t:ty, $wide:ty, $sign:ident) => {
		/// Apply operation `op` to operands taken from `data`.
		pub fn $name(op: u8, data: &[u8], out: &mut Vec<u8>) {
			let (a, data) = take!($t, data);
			let (b, data) = take!($t, data);
			// A third operand, from the bytes the shift is read from.
			let (c, _) = take!($t, data);
			let shift = data.first().copied().unwrap_or(0) as u32 % (2 * <$t>::BITS);
			let put = |out: &mut Vec<u8>, v: $t| {
				out.extend_from_slice(&(v as $wide).to_be_bytes());
				// A wide value equals its bytes read back only in its one form.
				out.push((<$t>::from_be_bytes(v.to_be_bytes()) == v) as u8);
			};
			let put_opt = |out: &mut Vec<u8>, v: Option<$t>| match v {
				Some(v) => {
					out.push(1);
					out.extend_from_slice(&(v as $wide).to_be_bytes());
				}
				None => out.push(0),
			};
			match op % 32 {
				0 => put(out, a + b),
				1 => put(out, a - b),
				2 => put(out, a * b),
				3 => put(out, a / b),
				4 => put(out, a % b),
				5 => put(out, a & b),
				6 => put(out, a | b),
				7 => put(out, a ^ b),
				8 => put(out, a << shift),
				9 => put(out, a >> shift),
				10 => put(out, !a),
				11 => put_opt(out, a.checked_add(b)),
				12 => put_opt(out, a.checked_sub(b)),
				13 => put_opt(out, a.checked_mul(b)),
				14 => put(out, a.saturating_add(b)),
				15 => put(out, a.saturating_sub(b)),
				16 => put(out, a.wrapping_add(b)),
				17 => put(out, a.wrapping_sub(b)),
				18 => out.extend_from_slice(&a.leading_ones().to_be_bytes()),
				19 => out.extend_from_slice(&a.leading_zeros().to_be_bytes()),
				20 => put(out, a.max(b)),
				21 => put(out, a.min(b)),
				22 => {
					out.push((a == b) as u8);
					out.push((a < b) as u8);
					out.push((a >= b) as u8);
					// Against a literal, which a wide value equals only in one form.
					out.push((a.wrapping_sub(b) == 0) as u8);
				}
				23 => {
					// Products nothing reads still overflow, of a sum that does too.
					let x = a + b;
					let _ = x * b;
					let _ = (x - b) * a;
					out.push(1);
				}
				24 => put(out, a.saturating_mul(b)),
				25 => put(out, a.wrapping_mul(b)),
				// Panics unless `b <= c`.
				26 => put(out, a.clamp(b, c)),
				27 => {
					if let Some(v) = div_ceil!($sign, $t, a, b) {
						put(out, v);
					}
				}
				28 => {
					if let Some(m) = is_multiple_of!($sign, a, b) {
						out.push(m as u8);
					}
				}
				29 => out.extend_from_slice(&a.to_le_bytes()),
				// Each width's edges, where a count is 0 or every bit, and through a `NonZero`.
				30 => {
					for v in [0, 1, <$t>::MAX, <$t>::MIN, a, a << shift % <$t>::BITS] {
						out.extend_from_slice(&v.trailing_zeros().to_be_bytes());
						match core::num::NonZero::new(v) {
							Some(n) => {
								out.extend_from_slice(&n.trailing_zeros().to_be_bytes());
								out.extend_from_slice(&n.leading_zeros().to_be_bytes());
							}
							None => out.push(0xff),
						}
					}
				}
				_ => {
					// Compound assignment, with constants folded beside variables.
					let mut x = a;
					x += b;
					x -= 1;
					x *= 2;
					x >>= 1;
					x <<= 1;
					x |= 1 << 3;
					x ^= b;
					x &= !(1 << 2);
					// By a constant, whose overflow check a signed type folds.
					x %= 7;
					x /= 3;
					put(out, x);
				}
			}
		}
	};
}

ops!(u8_op, u8, u128, unsigned);
ops!(u16_op, u16, u128, unsigned);
ops!(u32_op, u32, u128, unsigned);
ops!(u64_op, u64, u128, unsigned);
ops!(u128_op, u128, u128, unsigned);
ops!(usize_op, usize, u128, unsigned);
ops!(i8_op, i8, i128, signed);
ops!(i16_op, i16, i128, signed);
ops!(i32_op, i32, i128, signed);
ops!(i64_op, i64, i128, signed);
ops!(i128_op, i128, i128, signed);
ops!(isize_op, isize, i128, signed);

macro_rules! neg {
	($name:ident, $t:ty) => {
		/// Negate, which overflows at the minimum.
		pub fn $name(data: &[u8], out: &mut Vec<u8>) {
			let (a, _) = take!($t, data);
			out.extend_from_slice(&((-a) as i128).to_be_bytes());
		}
	};
}

neg!(i8_neg, i8);
neg!(i16_neg, i16);
neg!(i32_neg, i32);
neg!(i64_neg, i64);
neg!(i128_neg, i128);
neg!(isize_neg, isize);

/// A cast's bytes, then its text, which shows the sign the result reads with.
macro_rules! put_cast {
	($out:expr, $v:expr) => {{
		let v = $v;
		$out.extend_from_slice(&v.to_be_bytes());
		$out.extend_from_slice(v.to_string().as_bytes());
	}};
}

macro_rules! cast_to {
	($v:expr, $to:expr, $out:expr) => {{
		let v = $v;
		let out: &mut Vec<u8> = $out;
		match $to % 12 {
			0 => put_cast!(out, v as u8),
			1 => put_cast!(out, v as u16),
			2 => put_cast!(out, v as u32),
			3 => put_cast!(out, v as u64),
			4 => put_cast!(out, v as u128),
			5 => put_cast!(out, v as usize),
			6 => put_cast!(out, v as i8),
			7 => put_cast!(out, v as i16),
			8 => put_cast!(out, v as i32),
			9 => put_cast!(out, v as i64),
			10 => put_cast!(out, v as i128),
			_ => put_cast!(out, v as isize),
		}
	}};
}

/// Cast a value of type `from` to type `to`, both indexes into the twelve types.
pub fn cast(from: u8, to: u8, data: &[u8], out: &mut Vec<u8>) {
	match from % 12 {
		0 => cast_to!(take!(u8, data).0, to, out),
		1 => cast_to!(take!(u16, data).0, to, out),
		2 => cast_to!(take!(u32, data).0, to, out),
		3 => cast_to!(take!(u64, data).0, to, out),
		4 => cast_to!(take!(u128, data).0, to, out),
		5 => cast_to!(take!(usize, data).0, to, out),
		6 => cast_to!(take!(i8, data).0, to, out),
		7 => cast_to!(take!(i16, data).0, to, out),
		8 => cast_to!(take!(i32, data).0, to, out),
		9 => cast_to!(take!(i64, data).0, to, out),
		10 => cast_to!(take!(i128, data).0, to, out),
		_ => cast_to!(take!(isize, data).0, to, out),
	}
}

/// Conversions that check, and ones that cannot fail.
pub fn convert(data: &[u8], out: &mut Vec<u8>) {
	let (wide, data) = take!(u64, data);
	let (signed, _) = take!(i32, data);
	out.push(u8::try_from(wide).is_ok() as u8);
	match u32::try_from(wide) {
		Ok(v) => out.extend_from_slice(&v.to_be_bytes()),
		Err(_) => out.push(0xff),
	}
	match usize::try_from(signed) {
		Ok(v) => out.extend_from_slice(&v.to_be_bytes()),
		Err(_) => out.push(0xfe),
	}
	match i8::try_from(signed) {
		Ok(v) => out.extend_from_slice(&v.to_be_bytes()),
		Err(_) => out.push(0xfd),
	}
	let tripled: Result<u64, _> = (wide as u128 * 3).try_into();
	out.extend_from_slice(&tripled.unwrap_or(u64::MAX).to_be_bytes());
	let narrowed: Result<u16, _> = signed.try_into();
	out.extend_from_slice(&narrowed.unwrap_or(0xffff).to_be_bytes());
	let widened: u64 = u64::from(wide as u16);
	let into: i64 = signed.into();
	out.extend_from_slice(&widened.to_be_bytes());
	out.extend_from_slice(&into.to_be_bytes());
	out.push(true as u8 + false as u8);
	// Arrays written in place, with constant and sign bytes, and one behind a reference.
	let [a, b, c, d] = (signed as u32).to_be_bytes();
	out.extend_from_slice(&i8::from_be_bytes([a]).to_be_bytes());
	out.extend_from_slice(&u16::from_be_bytes([a, b]).to_be_bytes());
	out.extend_from_slice(&i16::from_be_bytes([a, b]).to_be_bytes());
	out.extend_from_slice(&u32::from_be_bytes([a, b, c, d]).to_be_bytes());
	out.extend_from_slice(&u32::from_be_bytes([0, b & 0x3f, c, d]).to_be_bytes());
	out.extend_from_slice(&i32::from_be_bytes([a, 0, c, d]).to_be_bytes());
	out.extend_from_slice(&u32::from_be_bytes([0, 0, 0, 0]).to_be_bytes());
	out.extend_from_slice(&u64::from_be_bytes([d, c, b, a, a, b, c, d]).to_be_bytes());
	let bytes = &[d, c, b, a];
	out.extend_from_slice(&i32::from_be_bytes(*bytes).to_be_bytes());
	// Halves joined back, which the runtime does in one call, of each width it takes.
	out.extend_from_slice(&from_halves((wide >> 32) as u32, wide as u32).to_be_bytes());
	let narrow = ((wide >> 40) as u8 as u64) << 32 | (wide as u16) as u64;
	out.extend_from_slice(&narrow.to_be_bytes());
	// Named constants past `i32::MAX`, which JS's 32-bit operators read as negative.
	out.extend_from_slice(&(HIGH >> 1).to_be_bytes());
	out.extend_from_slice(&(TOP >> 4).to_be_bytes());
}

const HIGH: u32 = u32::MAX;
const TOP: usize = 1 << 31;

/// A `u64` from its high and low 32 bits, as moq-net's varints join them.
fn from_halves(hi: u32, lo: u32) -> u64 {
	((hi as u64) << 32) | lo as u64
}
