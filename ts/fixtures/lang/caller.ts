// A hand-written caller of the translation, as moq's TypeScript is: it runs the harness,
// and prints a `Fault` with the `Display` and `Debug` that `printable` brings along,
// which nothing translated calls. `differential.ts` runs it beside native Rust's
// `harness::entry`, from the output directory.

import * as Fault from "./fmt.Fault.ts";
import { cases as named, run as harness, op } from "./harness.ts";
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
const loops = [0xf1, 0xf3, 0xf4, 0xf5, 0xf6, 0xf9, 0xfa].flatMap((b) => [Uint8Array.of(op("shapes::loops"), 0, b), Uint8Array.of(op("shapes::loops"), 0, b, 1)]);

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
const durationsOp = op("durations::durations");
const durations = [
	...[0n, 1n, 1n << 32n, (1n << 53n) - 1n, 1n << 53n, (1n << 64n) - 2n, (1n << 64n) - 1n].flatMap((secs) =>
		[0, 1, 999_999_999, 1_000_000_000, 0xffffffff].flatMap((nanos) => [0, 1].map((kind) => Uint8Array.of(durationsOp, kind, ...duration(secs, nanos)))),
	),
	...edges.flatMap((a) => edges.flatMap((b) => factors.flatMap((n) => [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13].map((kind) => Uint8Array.of(durationsOp, kind, ...a, ...b, ...n))))),
	// Each side of where minutes, hours, and a `u128` of nanoseconds pass `u64::MAX` seconds.
	...[2n ** 64n / 60n, 2n ** 64n / 3600n].flatMap((units) => [units, units + 1n].map((n) => wide(n << 64n))),
	...[(7n << 64n) + 999_999_999n, 2n ** 64n * 1_000_000_000n - 1n, 2n ** 64n * 1_000_000_000n, 2n ** 128n - 1n].map((n) => wide(n)),
];

/** `durations`' larger units, of a `u128` whose high half is also its `u64`. */
function wide(n: bigint): Uint8Array {
	const bytes = new Uint8Array(18);
	bytes.set([durationsOp, 12]);
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
const floatsOp = op("floats::floats");
/** How many cases the first byte picks among, modulo. */
const count = named().length;
const floats = [
	...Array.from({ length: 10 }, (_, kind) => doubles.flatMap((a) => doubles.map((b) => Uint8Array.of(floatsOp, kind, ...double(a), ...double(b), 1)))).flat(),
	// `from_secs_f64`, `mul_f64`, and `div_f64` panicking, and `Duration::MAX.mul_f64`.
	...[18, 28, 38].flatMap((kind) => doubles.map((a) => Uint8Array.of(floatsOp, kind, ...double(a), ...double(a), 3))),
	...wides.map((v) => {
		const bytes = new Uint8Array(16);
		new DataView(bytes.buffer).setBigUint64(0, BigInt.asUintN(64, v >> 64n));
		new DataView(bytes.buffer).setBigUint64(8, BigInt.asUintN(64, v));
		return Uint8Array.of(floatsOp, 2, ...bytes);
	}),
];

/** How many ULPs from native Rust's each `f64` of `floats`' kind 7 may be, which Rust leaves to libm. */
const ULPS = 2n;

export function within(input: Uint8Array, native: Uint8Array, translated: Uint8Array): boolean {
	if (input[0] % count !== floatsOp || input[1] % 10 !== 7 || native.length !== translated.length) return false;
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
 * The 64- and 128-bit cases over operands that random bytes rarely make: every pair of
 * values next to a power of two.
 */
function ints(): Uint8Array[] {
	const [u64, u128, i64, i128] = ["ints::u64_op", "ints::u128_op", "ints::i64_op", "ints::i128_op"].map(op);
	const [neg, cast, convert, fmtInts] = ["ints::neg", "ints::cast", "ints::convert", "fmt::ints"].map(op);
	const cases: Uint8Array[] = [];
	// 2^k - 1, 2^k, and 2^k + 1 for each k where a limb, a safe number, or a sign ends,
	// and values without that structure, whose quotients and remainders are not round.
	const edges = new Set<bigint>([
		1_000_000n,
		0x9e3779b97f4a7c15n,
		0x0123456789abcdefn,
		0xfedcba9876543210n,
		// Safe integers whose halves carry when multiplied, and a negative one.
		0x1d3c5b7a9f8e6dn,
		-0x1b2a3c4d5e6f71n & 0xffffffffffffffffn,
	]);
	for (const k of [0n, 1n, 16n, 21n, 31n, 32n, 33n, 52n, 53n, 54n, 63n, 64n]) {
		for (const delta of [-1n, 0n, 1n]) edges.add(((1n << k) + delta) & 0xffffffffffffffffn);
	}
	// Small negative numbers, -2^32, and where negative safe numbers end, as signed values.
	for (const k of [1n, 32n, 53n]) {
		for (const delta of [-1n, 0n, 1n]) edges.add((-(1n << k) + delta) & 0xffffffffffffffffn);
	}
	const values = [...edges];
	const be = (v: bigint) => {
		const bytes = new Uint8Array(8);
		new DataView(bytes.buffer).setBigUint64(0, v);
		return [...bytes];
	};
	// Shift amounts in and past each limb, and past the width, which panics.
	const shifts = [0, 1, 7, 31, 32, 33, 63, 64, 65, 95, 96, 127, 128, 200];
	let i = 0;
	for (const x of values) {
		for (const y of values) {
			// A 64-bit case's operands are `x` and `y`. A 128-bit case's are `x:y` and `u:v`,
			// whose high halves are equal a third of the time, so the low half decides.
			const u = i % 3 === 0 ? x : values[(i * 7) % values.length];
			const v = values[(i * 13 + 5) % values.length];
			const shift = shifts[i++ % shifts.length];
			for (let kind = 0; kind < 30; kind++) {
				for (const o of [u64, i64]) cases.push(Uint8Array.of(o, kind, ...be(x), ...be(y), shift));
				for (const o of [u128, i128]) cases.push(Uint8Array.of(o, kind, ...be(x), ...be(y), ...be(u), ...be(v), shift));
			}
			// `clamp` between two values of the sweep, whose third operand follows the shift's place.
			for (const o of [u64, i64]) cases.push(Uint8Array.of(o, 26, ...be(x), ...be(y), ...be(u)));
			for (const o of [u128, i128]) cases.push(Uint8Array.of(o, 26, ...be(x), ...be(y), ...be(u), ...be(v), ...be(y), ...be(x)));
			// Negating an `i64` and an `i128`.
			cases.push(Uint8Array.of(neg, 3, ...be(x)), Uint8Array.of(neg, 4, ...be(x), ...be(y)));
			// `x * y`, `checked_mul`, `saturating_mul`, and `wrapping_mul` at 128 bits, extended in
			// each type's sign: a product of two numbers, past 2^53, multiplies their halves.
			const signs = (v: bigint) => (v >> 63n === 1n ? 0xffffffffffffffffn : 0n);
			for (const kind of [2, 13, 24, 25]) {
				cases.push(Uint8Array.of(u128, kind, ...be(0n), ...be(x), ...be(0n), ...be(y)));
				cases.push(Uint8Array.of(i128, kind, ...be(signs(x)), ...be(x), ...be(signs(y)), ...be(y)));
			}
			// `x:y / d`, `x:y % d`, `div_ceil`, and `is_multiple_of` at 128 bits by a divisor
			// below 2^32 in magnitude, which divides limbs by a number, extended in each type's
			// sign: -1 overflows `i128::MIN`.
			for (const d of [3n, 1_000_000n, 0xffffffffn, -1n, -0xffffffffn]) {
				for (const kind of [3, 4, 27, 28]) {
					for (const o of [u128, i128]) cases.push(Uint8Array.of(o, kind, ...be(x), ...be(y), ...be(signs(d & 0xffffffffffffffffn)), ...be(d & 0xffffffffffffffffn)));
				}
			}
		}
		// Conversions, and halves joined back.
		cases.push(Uint8Array.of(convert, 0, ...be(x), ...be(x)));
		for (const y of [0n, 1n, 1n << 32n, 1n << 63n, 0xffffffffffffffffn]) {
			for (const from of [3, 4, 9, 10]) {
				// A cast to every type, and every format spec.
				for (let to = 0; to < 12; to++) cases.push(Uint8Array.of(cast, from, to, ...be(x), ...be(y)));
				cases.push(Uint8Array.of(fmtInts, from, ...be(x), ...be(y)));
			}
		}
	}
	return cases;
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
].map((bytes) => Uint8Array.of(op("utf8::decodes"), 0, ...bytes));

/**
 * `dyns::dyns` too short to run, with a `Tri` of each variant, at the ends of each side and
 * count, and boxing each kind of shape.
 */
const dyns = [[], [1, 2], [3, 0, 7], [255, 1, 255, 0, 1, 2, 3, 4, 5], [0, 3, 2, 9, 255, 6, 128]].map((bytes) =>
	Uint8Array.of(op("dyns::dyns"), 0, ...bytes),
);

/**
 * `cells` scripts of (op, operand) pairs on every kind, so each cell and value type: reads
 * and sets, a `None` and a `()` held, initializing from within initialization and
 * panicking in it, empty and full, and a cell replaced.
 */
const cellsOp = op("cells::cells");
const cellScripts = [
	[0, 0, 1, 5, 0, 0, 1, 6, 2, 7, 4, 0, 0, 0, 4, 0, 2, 9, 0, 0],
	[1, 3, 0, 0, 1, 4, 4, 0, 1, 6, 2, 3],
	[3, 4, 3, 5],
	[5, 3],
	[6, 3],
	[7, 1],
	[2, 1, 5, 3, 6, 3, 7, 1],
	[8, 4, 0, 0, 8, 5, 0, 0, 1, 3, 8, 6],
	[],
];
const cells = Array.from({ length: 256 }, (_, kind) => cellScripts.map((script) => Uint8Array.of(cellsOp, kind, ...script))).flat();

/**
 * `locks` scripts: writes through guards and reads, `get_mut`, a guard held while others
 * lock and unlock, and a second lock while it is held, which panics.
 */
const locks = [[0, 1, 6, 7, 2, 8, 3, 9, 3, 0, 1], [4, 5, 4, 5, 1], [4, 2, 3, 9, 1], [4, 0], [4, 4], [4]].map(
	(bytes) => Uint8Array.of(op("locks::locks"), 0, ...bytes),
);

export const cases: Uint8Array[] = [...loops, ...durations, ...ints(), ...utf8, ...floats, ...dyns, ...cells, ...locks];
