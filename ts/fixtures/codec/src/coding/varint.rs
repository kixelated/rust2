//! Variable-length integers: QUIC's two-bit length tag, and moq-transport's leading ones.
//!
//! A varint is a wire encoding of a plain `u64`, not a type. The codec works on the two
//! `u32` halves so it never needs 64-bit bitwise math, which a JavaScript `number`
//! cannot do.

use thiserror::Error;

use super::{DecodeError, EncodeError};
use crate::{Version, ietf, lite};

/// The number does not fit the target: a varint wire form or a narrower integer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Error)]
#[error("value out of range")]
pub struct BoundsExceeded;

/// The largest value the QUIC form can carry: `2^62 - 1`.
pub const MAX_QUIC: u64 = (1 << 62) - 1;

/// How a protocol version lays out a varint on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Form {
	/// QUIC's two-bit length tag, up to [`MAX_QUIC`].
	Quic,
	/// Leading one bits count the length, up to `u64::MAX`.
	LeadingOnes {
		/// Whether the 7-byte form (`1111110x`) is accepted on decode, which draft-17 forbids.
		seven: bool,
	},
}

impl From<lite::Version> for Form {
	fn from(version: lite::Version) -> Self {
		match version {
			lite::Version::Lite01
			| lite::Version::Lite02
			| lite::Version::Lite03
			| lite::Version::Lite04
			| lite::Version::Lite05
			| lite::Version::Lite06 => Self::Quic,
			// Lite-07 is only reached through its ALPN, so the form is known before the
			// first byte of any stream.
			_ => Self::LeadingOnes { seven: true },
		}
	}
}

impl From<ietf::Version> for Form {
	fn from(version: ietf::Version) -> Self {
		match version {
			ietf::Version::Draft14 | ietf::Version::Draft15 | ietf::Version::Draft16 => Self::Quic,
			ietf::Version::Draft17 => Self::LeadingOnes { seven: false },
			_ => Self::LeadingOnes { seven: true },
		}
	}
}

impl From<Version> for Form {
	fn from(version: Version) -> Self {
		match version {
			Version::Lite(v) => v.into(),
			Version::Ietf(v) => v.into(),
		}
	}
}

/// The high and low 32 bits: the only place the codec splits a `u64`.
const fn to_halves(value: u64) -> (u32, u32) {
	((value >> 32) as u32, value as u32)
}

/// The inverse of [`to_halves`].
const fn from_halves(hi: u32, lo: u32) -> u64 {
	((hi as u64) << 32) | lo as u64
}

/// The bytes `value` takes on the wire in `form`, or [`BoundsExceeded`] if it does not fit.
pub(crate) fn size(value: u64, form: Form) -> Result<usize, BoundsExceeded> {
	let (hi, lo) = to_halves(value);
	Ok(match form {
		Form::Quic if hi == 0 && lo < 1 << 6 => 1,
		Form::Quic if hi == 0 && lo < 1 << 14 => 2,
		Form::Quic if hi == 0 && lo < 1 << 30 => 4,
		Form::Quic if hi < 1 << 30 => 8,
		Form::Quic => return Err(BoundsExceeded),
		Form::LeadingOnes { .. } if hi == 0 && lo < 1 << 7 => 1,
		Form::LeadingOnes { .. } if hi == 0 && lo < 1 << 14 => 2,
		Form::LeadingOnes { .. } if hi == 0 && lo < 1 << 21 => 3,
		Form::LeadingOnes { .. } if hi == 0 && lo < 1 << 28 => 4,
		Form::LeadingOnes { .. } if hi < 1 << 3 => 5,
		Form::LeadingOnes { .. } if hi < 1 << 10 => 6,
		// The 7-byte form is skipped: one byte longer, but legal on every draft.
		Form::LeadingOnes { .. } if hi < 1 << 24 => 8,
		Form::LeadingOnes { .. } => 9,
	})
}

/// Append the minimal encoding of `value` in `form`.
///
/// Fails past [`MAX_QUIC`] in the QUIC form, writing nothing.
#[cfg_attr(target_arch = "wasm32", inline)]
#[cfg_attr(not(target_arch = "wasm32"), inline(always))]
pub(super) fn write(value: u64, form: Form, out: &mut Vec<u8>) -> Result<(), BoundsExceeded> {
	match form {
		Form::Quic => write_quic(value, out),
		Form::LeadingOnes { .. } => {
			write_leading_ones(value, out);
			Ok(())
		}
	}
}

// Each arm below is a fixed-size write or read, which is what keeps the codec as fast as
// a hand-rolled `put_u16`/`get_u32`. Natively the varint path is `inline(always)` from
// `Encoder::varint`/`Decoder::varint` down: left to LLVM's heuristics it stays a call
// whose `Result<u64, DecodeError>` goes through memory, which doubles the cost of a varint
// in a tight loop. wasm32 builds optimize for size, where forcing it grew moq-wasm ~10%
// gzipped, so they keep the heuristics. The QUIC read is an if-chain rather than a `match`
// on the tag: the jump table measured ~35% slower on a mixed-length stream.

#[cfg_attr(target_arch = "wasm32", inline)]
#[cfg_attr(not(target_arch = "wasm32"), inline(always))]
fn write_quic(value: u64, out: &mut Vec<u8>) -> Result<(), BoundsExceeded> {
	let (hi, lo) = to_halves(value);
	if hi == 0 && lo < 1 << 6 {
		out.push(lo as u8);
	} else if hi == 0 && lo < 1 << 14 {
		out.extend_from_slice(&(0x4000 | lo as u16).to_be_bytes());
	} else if hi == 0 && lo < 1 << 30 {
		out.extend_from_slice(&(0x8000_0000 | lo).to_be_bytes());
	} else if hi < 1 << 30 {
		let [a, b, c, d] = (0xc000_0000 | hi).to_be_bytes();
		let [e, f, g, h] = lo.to_be_bytes();
		out.extend_from_slice(&[a, b, c, d, e, f, g, h]);
	} else {
		return Err(BoundsExceeded);
	}
	Ok(())
}

#[cfg_attr(target_arch = "wasm32", inline)]
#[cfg_attr(not(target_arch = "wasm32"), inline(always))]
fn write_leading_ones(value: u64, out: &mut Vec<u8>) {
	let (hi, lo) = to_halves(value);
	let [a, b, c, d] = lo.to_be_bytes();
	if hi == 0 && lo < 1 << 7 {
		out.push(d);
	} else if hi == 0 && lo < 1 << 14 {
		out.extend_from_slice(&[0x80 | c, d]);
	} else if hi == 0 && lo < 1 << 21 {
		out.extend_from_slice(&[0xc0 | b, c, d]);
	} else if hi == 0 && lo < 1 << 28 {
		out.extend_from_slice(&[0xe0 | a, b, c, d]);
	} else if hi < 1 << 3 {
		out.extend_from_slice(&[0xf0 | hi as u8, a, b, c, d]);
	} else if hi < 1 << 10 {
		out.extend_from_slice(&[0xf8 | (hi >> 8) as u8, hi as u8, a, b, c, d]);
	} else if hi < 1 << 24 {
		// The 7-byte form is skipped: one byte longer, but legal on every draft.
		let [_, f, g, h] = hi.to_be_bytes();
		out.extend_from_slice(&[0xfe, f, g, h, a, b, c, d]);
	} else {
		let [e, f, g, h] = hi.to_be_bytes();
		out.extend_from_slice(&[0xff, e, f, g, h, a, b, c, d]);
	}
}

/// Decode a varint in `form` from the front of `buf`, returning it and the rest of `buf`.
#[cfg_attr(target_arch = "wasm32", inline)]
#[cfg_attr(not(target_arch = "wasm32"), inline(always))]
#[cfg_attr(not(feature = "async"), allow(dead_code, reason = "async-only"))]
pub(super) fn read(buf: &[u8], form: Form) -> Result<(u64, &[u8]), DecodeError> {
	match form {
		Form::Quic => read_quic(buf),
		Form::LeadingOnes { seven } => read_leading_ones(buf, seven),
	}
}

#[cfg_attr(target_arch = "wasm32", inline)]
#[cfg_attr(not(target_arch = "wasm32"), inline(always))]
fn read_quic(buf: &[u8]) -> Result<(u64, &[u8]), DecodeError> {
	let Some((&first, rest)) = buf.split_first() else {
		return Err(DecodeError::Short);
	};

	let be = u32::from_be_bytes;
	if first < 0x40 {
		Ok((first as u64, rest))
	} else if first < 0x80 {
		let ([a, b], rest) = buf.split_first_chunk().ok_or(DecodeError::Short)?;
		Ok((be([0, 0, a & 0x3f, *b]) as u64, rest))
	} else if first < 0xc0 {
		let ([a, b, c, d], rest) = buf.split_first_chunk().ok_or(DecodeError::Short)?;
		Ok((be([a & 0x3f, *b, *c, *d]) as u64, rest))
	} else {
		let ([a, b, c, d, lo @ ..], rest) = buf.split_first_chunk::<8>().ok_or(DecodeError::Short)?;
		Ok((from_halves(be([a & 0x3f, *b, *c, *d]), be(*lo)), rest))
	}
}

#[cfg_attr(target_arch = "wasm32", inline)]
#[cfg_attr(not(target_arch = "wasm32"), inline(always))]
#[cfg_attr(not(feature = "async"), allow(dead_code, reason = "async-only"))]
fn read_leading_ones(buf: &[u8], seven: bool) -> Result<(u64, &[u8]), DecodeError> {
	let Some((&first, rest)) = buf.split_first() else {
		return Err(DecodeError::Short);
	};

	let be = u32::from_be_bytes;
	Ok(match first.leading_ones() {
		0 => (first as u64, rest),
		1 => {
			let ([a, b], rest) = buf.split_first_chunk().ok_or(DecodeError::Short)?;
			(be([0, 0, a & 0x3f, *b]) as u64, rest)
		}
		2 => {
			let ([a, b, c], rest) = buf.split_first_chunk().ok_or(DecodeError::Short)?;
			(be([0, a & 0x1f, *b, *c]) as u64, rest)
		}
		3 => {
			let ([a, b, c, d], rest) = buf.split_first_chunk().ok_or(DecodeError::Short)?;
			(be([a & 0x0f, *b, *c, *d]) as u64, rest)
		}
		4 => {
			let ([a, lo @ ..], rest) = buf.split_first_chunk::<5>().ok_or(DecodeError::Short)?;
			(from_halves((a & 0x07) as u32, be(*lo)), rest)
		}
		5 => {
			let ([a, b, lo @ ..], rest) = buf.split_first_chunk::<6>().ok_or(DecodeError::Short)?;
			(from_halves(be([0, 0, a & 0x03, *b]), be(*lo)), rest)
		}
		// 1111110x: the 7-byte form, which draft-17 forbids.
		6 if !seven => return Err(DecodeError::InvalidValue),
		6 => {
			let ([a, b, c, lo @ ..], rest) = buf.split_first_chunk::<7>().ok_or(DecodeError::Short)?;
			(from_halves(be([0, a & 0x01, *b, *c]), be(*lo)), rest)
		}
		7 => {
			let ([_, b, c, d, lo @ ..], rest) = buf.split_first_chunk::<8>().ok_or(DecodeError::Short)?;
			(from_halves(be([0, *b, *c, *d]), be(*lo)), rest)
		}
		_ => {
			let ([_, hi @ .., e, f, g, h], rest) = buf.split_first_chunk::<9>().ok_or(DecodeError::Short)?;
			(from_halves(be(*hi), be([*e, *f, *g, *h])), rest)
		}
	})
}

/// Decode a QUIC varint (two-bit length tag) from the front of `r`.
pub fn decode_quic<R: bytes::Buf>(r: &mut R) -> Result<u64, DecodeError> {
	let Some(&first) = r.chunk().first() else {
		return Err(DecodeError::Short);
	};

	// Copy out so a varint split across chunks still decodes.
	let len = 1usize << (first >> 6);
	if r.remaining() < len {
		return Err(DecodeError::Short);
	}
	let mut buf = [0u8; 8];
	r.copy_to_slice(&mut buf[..len]);

	Ok(read_quic(&buf[..len])?.0)
}

/// Encode `value` as a QUIC varint (two-bit length tag).
///
/// Fails with [`EncodeError::BoundsExceeded`] past [`MAX_QUIC`], writing nothing.
pub fn encode_quic<W: bytes::BufMut>(value: u64, w: &mut W) -> Result<(), EncodeError> {
	let len = size(value, Form::Quic)?;
	if w.remaining_mut() < len {
		return Err(EncodeError::Short);
	}

	let (hi, lo) = to_halves(value);
	match len {
		1 => w.put_u8(lo as u8),
		2 => w.put_u16(0x4000 | lo as u16),
		4 => w.put_u32(0x8000_0000 | lo),
		_ => {
			w.put_u32(0xc000_0000 | hi);
			w.put_u32(lo);
		}
	}
	Ok(())
}

/// Map a signed value onto the unsigned range: `(n << 1) ^ (n >> 63)`.
///
/// Small magnitudes stay small (-1 -> 1, 1 -> 2, -2 -> 3, ...), and all of `i64` fits.
#[cfg_attr(not(feature = "async"), allow(dead_code, reason = "async-only"))]
pub(crate) const fn zigzag(signed: i64) -> u64 {
	((signed << 1) ^ (signed >> 63)) as u64
}

/// The inverse of [`zigzag`].
#[cfg_attr(not(feature = "async"), allow(dead_code, reason = "async-only"))]
pub(crate) const fn unzigzag(value: u64) -> i64 {
	((value >> 1) as i64) ^ -((value & 1) as i64)
}
