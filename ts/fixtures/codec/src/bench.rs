//! The workloads `just ts bench` times on every route, shaped like moq's own TypeScript
//! benchmarks: `js/net/bench/varint.ts` and `lite-varint.ts`. The first input byte picks
//! the workload and the second its wire format; an encode's output is the matching
//! decode's input. Op 6 runs iterator chains over its bytes instead, as moq-net's model
//! writes them.

use crate::coding::{Decode, DecodeError, Decoder, Encode, EncodeError, Encoder, Form, varint};
use crate::harness::{decode_error, encode_error, form, lite_version, put_u64, put_usize, take_u64};
use crate::lite;

/// Run one workload.
pub fn run(input: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	let Some((&op, data)) = input.split_first() else {
		return out;
	};
	let Some((&selector, data)) = data.split_first() else {
		return out;
	};

	match op {
		0 => encoded(encode_varints(data, form(selector), &mut out), &mut out),
		1 => decoded(decode_varints(data, form(selector), &mut out), &mut out),
		2 => encoded(encode_frames(data, lite_version(selector), &mut out), &mut out),
		3 => decoded(decode_frames(data, lite_version(selector), &mut out), &mut out),
		4 => encoded(encode_group(data, lite_version(selector), &mut out), &mut out),
		6 => iterate(selector, data, &mut out),
		_ => decoded(decode_group(data, lite_version(selector), &mut out), &mut out),
	}
	out
}

fn encoded(result: Result<(), EncodeError>, out: &mut Vec<u8>) {
	if let Err(err) = result {
		encode_error(out, &err);
	}
}

fn decoded(result: Result<(), DecodeError>, out: &mut Vec<u8>) {
	if let Err(err) = result {
		decode_error(out, &err);
	}
}

/// The same value, `count` times.
fn encode_varints(data: &[u8], form: Form, out: &mut Vec<u8>) -> Result<(), EncodeError> {
	let (count, data) = take_u64(data);
	let (value, _) = take_u64(data);
	let mut w = Encoder::new(out, form);
	for _ in 0..count as usize {
		w.varint(value)?;
	}
	Ok(())
}

/// Every varint on the wire: how many, and the last.
fn decode_varints(wire: &[u8], form: Form, out: &mut Vec<u8>) -> Result<(), DecodeError> {
	let mut r = Decoder::new(wire, form);
	let mut count = 0;
	let mut last = 0;
	while !r.is_empty() {
		last = r.varint()?;
		count += 1;
	}
	put_usize(out, count);
	put_u64(out, last);
	Ok(())
}

/// A group's FRAME headers without their payloads, as a lite publisher writes them: the
/// zigzag timestamp delta, then the size. The first frame has no delta and its own size,
/// and every `key_every`th frame after it is `key_size`, as moq's benchmark samples are.
fn encode_frames(data: &[u8], version: lite::Version, out: &mut Vec<u8>) -> Result<(), EncodeError> {
	let (count, data) = take_u64(data);
	let (delta, data) = take_u64(data);
	let (first, data) = take_u64(data);
	let (size, data) = take_u64(data);
	let (key_every, data) = take_u64(data);
	let (key_size, _) = take_u64(data);

	let (delta, key_every) = (delta as i64, key_every as usize);

	let mut w = Encoder::new(out, version.into());
	for n in 0..count as usize {
		let (delta, size) = match n {
			0 => (0, first),
			n if key_every != 0 && n % key_every == 0 => (delta, key_size),
			_ => (delta, size),
		};
		w.varint(varint::zigzag(delta))?;
		w.varint(size)?;
	}
	Ok(())
}

/// Every FRAME header on the wire: how many, their total size, and the last delta.
fn decode_frames(wire: &[u8], version: lite::Version, out: &mut Vec<u8>) -> Result<(), DecodeError> {
	let mut r = Decoder::new(wire, version.into());
	let mut count = 0;
	let mut total = 0;
	let mut last = 0;
	while !r.is_empty() {
		last = varint::unzigzag(r.varint()?);
		total += r.varint()?;
		count += 1;
	}
	put_usize(out, count);
	put_u64(out, total);
	put_u64(out, last as u64);
	Ok(())
}

/// One size-prefixed GROUP header, starting at the first frame.
fn encode_group(data: &[u8], version: lite::Version, out: &mut Vec<u8>) -> Result<(), EncodeError> {
	let (subscribe, data) = take_u64(data);
	let (sequence, _) = take_u64(data);
	let group = lite::Group {
		subscribe,
		sequence,
		frame_start: 0,
	};
	group.encode(&mut Encoder::new(out, version.into()), version)
}

/// One iterator chain over every byte, picked by `chain`, with closures that cost little
/// next to the iterator protocol they run through.
fn iterate(chain: u8, data: &[u8], out: &mut Vec<u8>) {
	match chain {
		0 => put_usize(out, data.iter().map(|&b| b >> 1).filter(|&b| b & 1 == 0).count()),
		1 => out.push(data.iter().any(|&b| b == 0xff) as u8),
		2 => {
			let odd: Vec<u8> = data.iter().copied().filter(|&b| b & 1 == 1).collect();
			put_usize(out, odd.len());
		}
		3 => put_u64(out, data.iter().map(|&b| b as u32).sum::<u32>() as u64),
		4 => put_u64(
			out,
			data.iter().skip(16).take(512).fold(0u32, |acc, &b| acc ^ b as u32) as u64,
		),
		_ => {
			let mut last = 0;
			for (i, &b) in data.iter().enumerate() {
				if b == 0x80 {
					last = i;
				}
			}
			put_usize(out, last);
		}
	}
}

fn decode_group(wire: &[u8], version: lite::Version, out: &mut Vec<u8>) -> Result<(), DecodeError> {
	let (group, used) = lite::Group::decode_slice(wire, version)?;
	put_u64(out, group.subscribe);
	put_u64(out, group.sequence);
	put_u64(out, group.frame_start);
	put_usize(out, used);
	Ok(())
}
