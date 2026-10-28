// A hand-written caller of the translation, as moq's TypeScript is: it runs the harness,
// and prints a `Fault` with the `Display` and `Debug` that `printable` brings along,
// which nothing translated calls. `differential.ts` runs it beside native Rust's
// `harness::entry`, from the output directory.

import * as Fault from "./fmt.Fault.ts";
import { run as harness } from "./harness.ts";
import * as rt from "./rt.ts";

export function run(input: rt.ByteSlice): rt.ByteVec {
	if (rt.byteSliceFirst(input) !== 0xff) return harness(input);
	const fault = Fault.new(rt.byteSliceRange(input, 1));
	const display: rt.Fmt = (f) => Fault.fmt$Display(fault, f);
	const debug: rt.Fmt = (f) => Fault.fmt$Debug(fault, f);
	const text = rt.fmtFormat([
		display,
		"|",
		rt.fmtSpec({ align: ">", width: 16 }, display),
		"|",
		debug,
		"|",
		rt.fmtSpec({ alternate: true }, debug),
	]);
	return rt.byteVecFrom(new TextEncoder().encode(text));
}

/** `shapes::loops` with each first byte that one of its cases of `!` takes, with one more byte and without. */
const loops = [0xf1, 0xf3, 0xf4, 0xf5, 0xf6, 0xf9, 0xfa].flatMap((b) => [Uint8Array.of(16, 0, b), Uint8Array.of(16, 0, b, 1)]);

/** A `Duration`'s bytes as `durations` takes them: its seconds, then its nanoseconds. */
function duration(secs: bigint, nanos: number): number[] {
	const bytes = new Uint8Array(12);
	const view = new DataView(bytes.buffer);
	view.setBigUint64(0, secs);
	view.setUint32(8, nanos);
	return [...bytes];
}

/**
 * `durations` where seconds leave a safe integer or reach `u64::MAX`, nanoseconds carry,
 * a sum carries them past it, and a factor makes the nanoseconds' product or a
 * quotient's remainder pass 2^53.
 */
const edges = [0n, 1n, (1n << 53n) - 1n, 1n << 53n, (1n << 64n) - 1n].flatMap((secs) => [0, 1, 999_999_999].map((nanos) => duration(secs, nanos)));
const factors = [0, 1, 3, 1000, 9_007_199, 1_000_000_000, 0xffffffff].map((n) => [n >>> 24, (n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff]);
const durations = [
	...[0n, 1n, 1n << 32n, (1n << 53n) - 1n, 1n << 53n, (1n << 64n) - 2n, (1n << 64n) - 1n].flatMap((secs) =>
		[0, 1, 999_999_999, 1_000_000_000, 0xffffffff].flatMap((nanos) => [0, 1].map((kind) => Uint8Array.of(47, kind, ...duration(secs, nanos)))),
	),
	...edges.flatMap((a) => edges.flatMap((b) => factors.flatMap((n) => [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13].map((kind) => Uint8Array.of(47, kind, ...a, ...b, ...n))))),
	// Each side of where minutes, hours, and a `u128` of nanoseconds pass `u64::MAX` seconds.
	...[2n ** 64n / 60n, 2n ** 64n / 3600n].flatMap((units) => [units, units + 1n].map((n) => wide(n << 64n))),
	...[(7n << 64n) + 999_999_999n, 2n ** 64n * 1_000_000_000n - 1n, 2n ** 64n * 1_000_000_000n, 2n ** 128n - 1n].map((n) => wide(n)),
];

/** `durations`' larger units, of a `u128` whose high half is also its `u64`. */
function wide(n: bigint): Uint8Array {
	const bytes = new Uint8Array(18);
	bytes.set([47, 12]);
	const view = new DataView(bytes.buffer);
	view.setBigUint64(2, n >> 64n);
	view.setBigUint64(10, n & 0xffff_ffff_ffff_ffffn);
	return bytes;
}

/** A double's bits, big-endian, as `floats` takes them. */
function double(x: number | bigint): number[] {
	const bytes = new Uint8Array(8);
	const view = new DataView(bytes.buffer);
	if (typeof x === "bigint") view.setBigUint64(0, x);
	else view.setFloat64(0, x);
	return [...bytes];
}

/**
 * `floats`' operands: each side of a half, of 2^52 and 2^53, where each integer type
 * saturates, of a nanosecond and `u64::MAX` seconds, subnormals, infinities, and NaNs,
 * one of them negative and one with a payload.
 */
const doubles: (number | bigint)[] = [
	0, -0, 0.5, -0.5, 1.5, 2.5, -2.5, 0.49999999999999994, 1, -1, 3, 0.1, 1 / 3, 2 ** 52 - 0.5, 2 ** 52, 2 ** 53, 2 ** 53 + 2,
	255.5, -128.5, 2147483647.5, -2147483648.5, 4294967295.9, 2 ** 63, -(2 ** 63), -(2 ** 63) - 2048, 2 ** 64, 2 ** 64 - 2048,
	2 ** 127, 2 ** 128, -(2 ** 128), 1e-9, 0.5e-9, 1.5e-9, 0.999999999, 0.9999999995, 1.0000000005, 4.656612873077393e-10,
	1e-310, 5e-324, 2.2250738585072014e-308, Number.MAX_VALUE, -Number.MAX_VALUE, Infinity, -Infinity, NaN,
	0xfff8000000000000n, 0x7ff0000000000001n,
];
/** Integers whose `f64` rounds, as `floats`' kind 2 takes them: 16 bytes of a `u128`. */
const wides = [0n, 1n, (1n << 53n) + 1n, (1n << 53n) + 3n, (1n << 54n) + 2n, (1n << 63n) + 1025n, (1n << 64n) - 1n, (1n << 64n) - 1025n, (1n << 64n) + (1n << 11n) + 1n, (1n << 127n) + 1n, (1n << 128n) - 1n, (1n << 128n) - (1n << 74n), -(1n << 100n) - 1n];
const floats = [
	...Array.from({ length: 10 }, (_, kind) => doubles.flatMap((a) => doubles.map((b) => Uint8Array.of(58, kind, ...double(a), ...double(b), 1)))).flat(),
	// `from_secs_f64`, `mul_f64`, and `div_f64` panicking, and `Duration::MAX.mul_f64`.
	...[18, 28, 38].flatMap((kind) => doubles.map((a) => Uint8Array.of(58, kind, ...double(a), ...double(a), 3))),
	...wides.map((v) => {
		const bytes = new Uint8Array(16);
		new DataView(bytes.buffer).setBigUint64(0, BigInt.asUintN(64, v >> 64n));
		new DataView(bytes.buffer).setBigUint64(8, BigInt.asUintN(64, v));
		return Uint8Array.of(58, 2, ...bytes);
	}),
];

/** How many ULPs from native Rust's each `f64` of `floats`' kind 7 may be, which Rust leaves to libm. */
const ULPS = 2n;

export function within(input: Uint8Array, native: Uint8Array, translated: Uint8Array): boolean {
	if (input[0] % 60 !== 58 || input[1] % 10 !== 7 || native.length !== translated.length) return false;
	const [a, b] = [native, translated].map((v) => new DataView(v.buffer, v.byteOffset, v.length));
	// Bits as a line through zero, so the distance between two is how many doubles apart they are.
	const line = (v: bigint) => (v < 0n ? -(v & 0x7fffffffffffffffn) : v);
	for (let i = 0; i < native.length; i += 8) {
		const d = line(a.getBigInt64(i)) - line(b.getBigInt64(i));
		if (d > ULPS || d < -ULPS) return false;
	}
	return true;
}

/**
 * `utf8::decodes` of raw bytes: a byte order mark, which std keeps, and the sequences at
 * each edge of what it takes: surrogates, overlongs, past U+10FFFF, and cut short at the
 * end or by another byte.
 */
const utf8 = [
	[0xef, 0xbb, 0xbf, 0x61],
	[0xed, 0x9f, 0xbf, 0xed, 0xa0, 0x80, 0xed, 0xbf, 0xbf],
	[0xc0, 0x80, 0xc1, 0xbf, 0xc2, 0x80],
	[0xe0, 0x80, 0x80, 0xe0, 0x9f, 0xbf, 0xe0, 0xa0, 0x80],
	[0xf0, 0x80, 0x80, 0x80, 0xf0, 0x8f, 0xbf, 0xbf, 0xf0, 0x90, 0x80, 0x80],
	[0xf4, 0x8f, 0xbf, 0xbf, 0xf4, 0x90, 0x80, 0x80, 0xf5, 0x80, 0x80, 0x80],
	[0xf0, 0x9f, 0x98, 0x80, 0xf0, 0x9f, 0x98, 0x41, 0xe2, 0x82, 0x41, 0xf0, 0x9f, 0x98],
	[0xe2, 0x82],
	[0xc2],
	[0x80, 0xf8, 0xff],
].map((bytes) => Uint8Array.of(57, 0, ...bytes));

export const cases: Uint8Array[] = [...loops, ...durations, ...utf8, ...floats];
