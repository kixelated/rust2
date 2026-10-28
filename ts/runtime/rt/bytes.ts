// `bytes::Bytes`, for rust2-ts output: immutable bytes that its clones and slices share,
// as a `&[u8]` shares them, so a `Bytes` is an `rt.ByteSlice`. `rt.ts` re-exports all of it.

import { ByteSlice, type FmtResult, type Formatter, fmtWriteStr, panic, type RangeInclusive } from "../rt.ts";
import type { usize } from "./int.ts";

/** `bytes::Bytes`. */
export type Bytes = ByteSlice;

/** `Bytes::new`. Its bytes are made each time, as a bundler keeps a constant object. */
export function bytesNew(): Bytes {
	return new ByteSlice(new Uint8Array(0), 0, 0);
}

/** `Bytes::copy_from_slice`: a copy, which later writes to `data` don't change. */
export function bytesCopyFromSlice(data: ByteSlice): Bytes {
	return new ByteSlice(data.data.slice(data.start, data.start + data.length), 0, data.length);
}

/** `Bytes::slice` of `begin..end`, `end` defaulting to the length, panicking as `bytes` does. */
export function bytesSlice(b: Bytes, begin: usize, end: usize = b.length as usize): Bytes {
	if (begin > end) panic(`range start must not be greater than end: ${begin} <= ${end}`);
	if (end > b.length) panic(`range end out of bounds: ${end} <= ${b.length}`);
	return new ByteSlice(b.data, b.start + begin, end - begin);
}

/** `Bytes::slice` of `a..=b`, which an exhausted range ends before. */
export function bytesSliceInclusive(b: Bytes, range: RangeInclusive<usize>): Bytes {
	if (range.exhausted) return bytesSlice(b, range.lo, range.hi);
	if (range.hi === 0xffffffff) panic("out of range");
	return bytesSlice(b, range.lo, (range.hi + 1) as usize);
}

/** `Bytes::slice` of `..=end`, which can't end past `usize::MAX`. */
export function bytesSliceToInclusive(b: Bytes, end: usize): Bytes {
	if (end === 0xffffffff) panic("out of range");
	return bytesSlice(b, 0 as usize, (end + 1) as usize);
}

/** `Debug`: a byte string literal, `b"\x00ab"`, written a piece at a time as `bytes` does. */
export function bytesDebug(b: Bytes, f: Formatter): FmtResult {
	let r = fmtWriteStr(f, 'b"');
	for (let i = 0; i < b.length && r.ok; i++) r = fmtWriteStr(f, escape(b.data[b.start + i]));
	return r.ok ? fmtWriteStr(f, '"') : r;
}

function escape(byte: number): string {
	switch (byte) {
		case 0x0a:
			return "\\n";
		case 0x0d:
			return "\\r";
		case 0x09:
			return "\\t";
		case 0x5c:
		case 0x22:
			return `\\${String.fromCharCode(byte)}`;
		case 0:
			return "\\0";
	}
	if (byte >= 0x20 && byte < 0x7f) return String.fromCharCode(byte);
	return `\\x${byte.toString(16).padStart(2, "0")}`;
}
