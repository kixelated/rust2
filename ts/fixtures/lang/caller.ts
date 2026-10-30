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

/**
 * `floats::formats`' doubles: ties at each precision, which std rounds half to even on the
 * exact value, carries through nines, each side of where `Debug` takes an exponent, the
 * shortest digits that are not the nearest ones, and the extremes.
 */
const formatted: (number | bigint)[] = [
	...doubles,
	0.125, 0.375, 0.625, 0.875, 0.0625, 3.5, 4.5, 9.5, 99.5, 999.5, 0.95, 9.995, 1.25, 1.35, 2.675, 1e-4, 9.999999999999999e-5,
	1e16, 9999999999999998, 1e21, 1e22, 1e23, 5e-324, 2.225073858507201e-308, 123456789.125, 1.5e300, -1.5e-300, 2 ** -1022 * 1.5,
	0.1 + 0.2, 100, 1e15, 1e-7, 0.000123, -9.5, -0.5,
];
const formatsOp = op("floats::formats");
const formats = [
	...formatted.flatMap((a) => Array.from({ length: 22 }, (_, p) => Uint8Array.of(formatsOp, 0, ...double(a), p))),
	...formatted.flatMap((a) => [1, 3, 4].map((kind) => Uint8Array.of(formatsOp, kind, ...double(a), 0))),
	// Seeds for `parses`' decimals, a signaling NaN's bits among them, which a seed keeps.
	...doubles.map((a) => Uint8Array.of(op("floats::parses"), 2, ...double(a))),
	...formatted.flatMap((a) => [[1, 2], [2, 1], [NaN, 1], [1, NaN], [-0, 0], [0, -0]].map(([b, c]) => Uint8Array.of(formatsOp, 2, ...double(a), ...double(b), ...double(c)))),
];

/**
 * `floats::parses`' texts: std's grammar at each edge, past each end of a double's range,
 * and digits that round half to even, or just off it past 19 digits.
 */
const parsed = [
	"", "+", "-", ".", "+.", "-.", "e5", ".e5", "1e", "1e+", "1e-", "1.e5", ".5", "5.", "-.5e-3", "+1", "++1", "+-1", "1_000", "0x10", " 1",
	"1 ", "1,5", "1.5.5", "1e5e5", "1E+05", "1e400", "-1e400", "1e-400", "1e99999999999999999999", "0e99999999999", "-0", "+0.0e-5",
	"0000.0000", "inf", "-inf", "+inf", "infinity", "INFINITY", "InFiNiTy", "infin", "in", "infinityy", "nan", "NaN", "-nan", "+NAN",
	"nan1", "Infinity", "i\u0307nf", "\u0131nf", "2.2250738585072011e-308", "2.2250738585072012e-308", "4.9406564584124654e-324",
	"2.4703282292062327e-324", "2.4703282292062328e-324", "1.7976931348623157e308", "1.7976931348623158e308", "1.7976931348623159e308",
	"9007199254740993", "9007199254740993.0000000000000000001", "9007199254740992.9999999999999999999", "123456789012345678901234567890",
	"0.1000000000000000055511151231257827021181583404541015625", "1e23", "8.98846567431158e307", `0.${"0".repeat(400)}1e400`,
	`1${"0".repeat(800)}e-800`, `${"9".repeat(800)}e-1100`, "1e65535", `1e-65536`, `0.${"0".repeat(70000)}1e70000`,
].map((text) => Uint8Array.of(op("floats::parses"), 0, ...new TextEncoder().encode(text)));

/** A single's bits, big-endian, as `singles` takes them. */
function single(x: number | bigint): number[] {
	const bytes = new Uint8Array(4);
	const view = new DataView(bytes.buffer);
	if (typeof x === "bigint") view.setUint32(0, Number(x));
	else view.setFloat32(0, x);
	return [...bytes];
}

/**
 * `singles`' operands: each side of a half, of 2^23 and 2^24, where each integer type
 * saturates, where a single's range and its normals end, the bounds of `Debug`'s
 * exponent, infinities, and NaNs, one negative, one with a payload, and one signaling.
 */
const singlesValues: (number | bigint)[] = [
	0, -0, 0.5, -0.5, 1.5, 2.5, -2.5, 0.49999997, 1, -1, 3, 0.1, 1 / 3, 2 ** 23 - 0.5, 2 ** 23, 2 ** 24, 2 ** 24 + 2, 255.5, -128.5,
	2147483520, 2 ** 31, -(2 ** 31), 4294967040, 2 ** 32, 2 ** 63, -(2 ** 63), 2 ** 64, 2 ** 127, 1e-9, 0.99999994, 1.0000001, 1e-4, 1e16,
	1e-40, 2 ** -149, 2 ** -126, 3.4028234663852886e38, -3.4028234663852886e38, Infinity, -Infinity, NaN, 0xffc00000n, 0x7fc00123n, 0x7f800001n,
];
const singlesOp = op("singles::singles");
const singlesCases = [
	...Array.from({ length: 10 }, (_, kind) => singlesValues.flatMap((a) => singlesValues.map((b) => Uint8Array.of(singlesOp, kind, ...single(a), ...single(b), 1)))).flat(),
	// `from_secs_f32`, `mul_f32`, and `div_f32` panicking, and `Duration::MAX.mul_f32`.
	...[18, 28, 38].flatMap((kind) => singlesValues.map((a) => Uint8Array.of(singlesOp, kind, ...single(a), ...single(a), 3))),
	// Integers whose `f32` rounds once where through an `f64` it would round twice: past
	// 2^53, a bit just past halfway between two singles.
	...wides.concat([(1n << 60n) + (1n << 36n) + 1n, (1n << 64n) - (1n << 39n) - 1n, (1n << 127n) + (1n << 103n) + 1n, (1n << 24n) + 1n]).map((v) => {
		const bytes = new Uint8Array(16);
		new DataView(bytes.buffer).setBigUint64(0, BigInt.asUintN(64, v >> 64n));
		new DataView(bytes.buffer).setBigUint64(8, BigInt.asUintN(64, v));
		return Uint8Array.of(singlesOp, 2, ...bytes);
	}),
];

/** `singles::formats`' singles: ties at each precision, carries through nines, and the shortest digits at each end. */
const singlesFormatted: (number | bigint)[] = [
	...singlesValues,
	0.125, 0.375, 3.5, 9.5, 99.5, 0.95, 9.995, 1.25, 1.35, 2.675, 9.999999e-5, 1e15, 9999999e9, 1e-7, 123456.79, 16777215, 7.006492e-46, 1.17549421e-38, 8388609,
];
const singlesFormatsOp = op("singles::formats");
const singlesParsesOp = op("singles::parses");
const singlesFormats = [
	...singlesFormatted.flatMap((a) => Array.from({ length: 12 }, (_, p) => Uint8Array.of(singlesFormatsOp, 0, ...single(a), p))),
	...singlesFormatted.flatMap((a) => [1, 3, 4].map((kind) => Uint8Array.of(singlesFormatsOp, kind, ...single(a), 0))),
	...singlesFormatted.flatMap((a) => [[1, 2], [2, 1], [NaN, 1], [1, NaN], [-0, 0], [0, -0]].map(([b, c]) => Uint8Array.of(singlesFormatsOp, 2, ...single(a), ...single(b), ...single(c)))),
	...singlesValues.map((a) => Uint8Array.of(singlesParsesOp, 2, ...single(a))),
];

/**
 * `singles::parses`' texts: std's grammar, past each end of a single's range, and
 * decimals within a hair of halfway between two singles, which rounding through the
 * nearest double would round twice.
 */
const singlesParsed = [
	"", "+", "-", ".", "e5", "1e", "1.e5", ".5", "+1", "1_000", " 1", "1e40", "-1e40", "1e-50", "-0", "inf", "-infinity", "NaN", "nan1",
	"3.4028235e38", "3.4028236e38", "3.40282357e38", "3.4028235677973366e38", "3.4028235677973367e38", "1.4e-45", "7e-46",
	"7.006492321624085354618647916449580656401e-46", "7.006492321624085354618647916449580656402e-46", "1.1754942e-38", "1.17549435e-38",
	"16777217", "16777217.000000000001", "16777218.99999999999", "16777219", "1.00000005960464477539062499", "1.000000059604644775390625",
	"1.00000005960464477539062501", "1.0000000596046448", "0.1", "1e10", "1234567e-10", "12345678e-10", "9007199254740993",
	`1${"0".repeat(60)}e-60`, `0.${"0".repeat(60)}1e61`, "1e65535", "1e-65536",
].map((text) => Uint8Array.of(singlesParsesOp, 0, ...new TextEncoder().encode(text)));

/** How many ULPs from native Rust's each float of `floats`' and `singles`' kind 7 may be, which Rust leaves to libm. */
const ULPS = 2n;

export function within(input: Uint8Array, native: Uint8Array, translated: Uint8Array): boolean {
	const width = input[0] % count === floatsOp ? 8 : input[0] % count === singlesOp ? 4 : 0;
	if (width === 0 || input[1] % 10 !== 7 || native.length !== translated.length) return false;
	const [a, b] = [native, translated].map((v) => new DataView(v.buffer, v.byteOffset, v.length));
	const read = (v: DataView, i: number) => (width === 8 ? v.getBigInt64(i) : BigInt(v.getInt32(i)));
	// Bits as a line through zero, so the distance between two is how many floats apart they are.
	const magnitude = (1n << BigInt(width * 8 - 1)) - 1n;
	const line = (v: bigint) => (v < 0n ? -(v & magnitude) : v);
	for (let i = 0; i < native.length; i += width) {
		const d = line(read(a, i)) - line(read(b, i));
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
			for (let kind = 0; kind < 31; kind++) {
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
 * `dyns::shared` too short to run, and with `Arc`s of each type, the last count of each at
 * zero and at the top of its range.
 */
const shared = [[], [1], [0, 0], [255, 255, 0, 1, 2, 3, 4, 5], [7, 0xff, 255, 254, 253]].map((bytes) =>
	Uint8Array.of(op("dyns::shared"), 0, ...bytes),
);

/**
 * `locks` scripts: writes through guards and reads, `get_mut`, and a lock matched.
 */
const locks = [[0, 1, 5, 6, 2, 7, 3, 8, 3, 0, 1], [4, 0, 4, 1], [2, 9, 2, 10, 4, 1], [3, 3, 4, 0]].map(
	(bytes) => Uint8Array.of(op("locks::locks"), 0, ...bytes),
);

/**
 * `statics::onces` scripts: a `Once` forced, called, and forced again, which drops the
 * function it never runs, each static's, each `call_once` and `call_once_force` run again
 * from within itself, and crossbeam-epoch's `OnceLock` of each kind.
 */
const onces = [[1, 0, 1, 7], [0, 6, 1, 2, 3, 4, 5, 11], [2, 14, 8], [9], [3, 15], [4, 17, 23, 29, 4]].map((bytes) => Uint8Array.of(op("statics::onces"), 0, ...bytes));

/**
 * `bounds` with each kind of bound at each end, over every pair of values around and past
 * four items, so ranges are empty, backwards, equal, and out of bounds, and of no items.
 */
const boundsOp = op("bounds::bounds");
const bounds = Array.from({ length: 6 * 9 * 36 }, (_, n) => {
	const [kind, ends, at] = [n % 6, Math.floor(n / 6) % 9, Math.floor(n / 54)];
	const [a, b] = [at % 6, Math.floor(at / 6)];
	return [Uint8Array.of(boundsOp, kind, ends % 3, Math.floor(ends / 3), a, b, 1, 3, 2, 5), Uint8Array.of(boundsOp, kind, ends % 3, Math.floor(ends / 3), a, b)];
}).flat();

/**
 * `pointers::cells` scripts: each operation on every cell, a number wrapping past its top, a
 * struct assigned through `get` and written over, and cells replaced or made and taken out.
 */
const unsafeCells = [
	[0, 248, 1, 9, 2, 10, 3, 11, 4, 12, 5, 13, 6, 14, 7, 15],
	[4, 12, 4, 5, 253, 7, 255, 0],
	[3, 251, 6, 3, 2, 2, 254],
	[],
].map((bytes) => Uint8Array.of(op("pointers::cells"), 0, ...bytes));

/**
 * `strs::edits` of each edit on `aé\u{800}😀`, a char of each UTF-8 width, at every byte
 * offset into it and up to two past its end, each made a raw offset by its byte's bits.
 */
const editsOp = op("strs::edits");
const edits = Array.from({ length: 14 * 16 * 16 }, (_, i) => Uint8Array.of(editsOp, 4, 2, 7, 9, 14, i >> 8, (i >> 4) & 15, i & 15));

/**
 * `subslices` of each kind over zero to four items, at every pair of bounds up to one past
 * the end, so parts are empty, at the end, backwards, and out of bounds.
 */
const subslicesOp = op("slices::subslices");
const subslices = [0, 1, 2, 3, 4].flatMap((n) =>
	Array.from({ length: 8 * (n + 2) ** 2 }, (_, i) => {
		const [kind, a, b] = [i % 8, Math.floor(i / 8) % (n + 2), Math.floor(i / 8 / (n + 2))];
		return Uint8Array.of(subslicesOp, kind, a, b, ...[0x31, 0xc2, 0x07, 0xfe].slice(0, n));
	}),
);

/**
 * `nested::consumers`, whose first byte is also `nth`'s index: of no items, one `Some` or
 * `None`, `None` before `Some`, `nth` past the end, only `None`s, keys all tied, and
 * `find` stopping early.
 */
const consumers = [[], [6], [5], [1, 2], [7, 2], [1, 3, 5], [2, 6, 10, 14], [4, 3, 0x90, 0x92, 5]].map((bytes) => Uint8Array.of(op("nested::nested"), 3, ...bytes));

/**
 * `nested::closures`, whose bytes `pick` each `None` (0 mod 3), `Some(None)` (1), or
 * `Some(Some(_))` (2): of no bytes, only `None`s, only `Some(None)`s, `Some(None)` before
 * a `None` and after it, a seed of each, and a `successors` whose size hint reserves past
 * `isize::MAX`.
 */
const closures = [[], [3], [0, 3], [1], [4, 7, 1], [1, 2, 0, 4, 5], [4, 0, 1, 3, 5], [2, 1, 4, 3, 8], [5, 4], [0, 1, 2], [0xf0], [0xf1]].map((bytes) => Uint8Array.of(op("nested::nested"), 5, ...bytes));

/**
 * `nested::elements`, whose first byte is also the index and whose odd bytes are each a
 * `None` element: of no elements, one of each, only `None`s, and an index in range and
 * past the end of each mix.
 */
const elements = [[], [1], [2], [1, 3], [3, 1, 5], [0, 1, 2], [2, 6, 1, 8], [5, 4, 3, 0x10, 7, 0x40], [7, 0x22, 9, 0x22, 1, 0x80, 3, 6]].map((bytes) => Uint8Array.of(op("nested::nested"), 6, ...bytes));

/**
 * `ranges` of each kind, and each kind of `Bound` at the end, over zero to four items, at
 * every pair of bounds up to one past the end and at `usize::MAX`.
 */
const rangesOp = op("slices::ranges");
const ranges = [0, 1, 2, 3, 4].flatMap((n) => {
	const ends = [...Array(n + 2).keys(), 0xff];
	return Array.from({ length: 24 * ends.length ** 2 }, (_, i) => {
		const [kind, a, b] = [i % 24, ends[Math.floor(i / 24) % ends.length], ends[Math.floor(i / 24 / ends.length)]];
		return Uint8Array.of(rangesOp, kind, a, b, ...[0x31, 0xc2, 0x07, 0xfe].slice(0, n));
	});
});

/**
 * `iters::reducers` of no items, one, one repeated, which `next_if_eq` takes, and keys
 * that all tie.
 */
const reducers = [[], [9], [9, 9], [8, 4, 12, 0x40, 0x80]].map((bytes) => Uint8Array.of(op("iters::reducers"), 0, ...bytes));

/**
 * `edits` of each kind over zero to four items, at every three bounds up to one past the
 * end, so parts are empty, at the end, backwards, and out of bounds.
 */
const editsSlicesOp = op("slices::edits");
const sliceEdits = [0, 1, 2, 3, 4].flatMap((n) =>
	Array.from({ length: 8 * (n + 2) ** 3 }, (_, i) => {
		const [kind, a, b, c] = [i % 8, Math.floor(i / 8) % (n + 2), Math.floor(i / 8 / (n + 2)) % (n + 2), Math.floor(i / 8 / (n + 2) ** 2)];
		return Uint8Array.of(editsSlicesOp, kind, a, b, c, ...[0x31, 0xc2, 0x07, 0xfe].slice(0, n));
	}),
);

/**
 * `strs::drains` of each range and way to end the drain on `aé\u{800}😀`, at every byte
 * offset into it and up to two past its end, reading none, two from either end, or three
 * from the back.
 */
const drainsOp = op("strs::drains");
const drains = Array.from({ length: 32 * 13 * 13 }, (_, i) => {
	const [form, a, b] = [i % 32, Math.floor(i / 32) % 13, Math.floor(i / 416)];
	return Uint8Array.of(drainsOp, 4, 2, 7, 9, 14, form, a, b, [0, 0x0a, 0x1f][(form + a + b) % 3]);
});

/** `strs::iters` of each kind, on no text, text of each UTF-8 width, and parts between `/`s, empty ones too. */
const iters = Array.from({ length: 12 }, (_, kind) => [[], [2, 7, 9, 14], [0, 2, 0, 0, 7, 9], [14, 0]].map((data) => Uint8Array.of(op("strs::iters"), kind, ...data))).flat();

/** `strs::hinted` of each adaptor and each pair of range lengths, each way, on `aé` alone. */
const hintedOp = op("strs::hinted");
const hinted = Array.from({ length: 10 * 7 * 21 }, (_, i) => Uint8Array.of(hintedOp, i % 10, Math.floor(i / 10) % 7, Math.floor(i / 70)));

/**
 * `hints::hints` of each iterator, each pair of lengths, and each of its arms, read three
 * times each way, with as many bytes as the first length's index.
 */
const hintsOp = op("hints::hints");
const hints = Array.from({ length: 24 * 8 * 8 * 8 * 2 }, (_, i) => {
	const [kind, n, m, arm, back] = [i % 24, Math.floor(i / 24) % 8, Math.floor(i / 192) % 8, Math.floor(i / 1536) % 8, Math.floor(i / 12288)];
	const steps = (arm << 5) | ((back ? 0b101 : 0b010) << 2) | 3;
	return Uint8Array.of(hintsOp, kind, n, m, steps, ...Array.from({ length: n }, (_, b) => b * 37));
});

/** `hints::reserves` of each collection and element type, each shape, length, and limit on what it reads. */
const reservesOp = op("hints::reserves");
const reserves = Array.from({ length: 13 * 5 * 5 * 4 * 5 }, (_, i) =>
	Uint8Array.of(reservesOp, i % 65, Math.floor(i / 65) % 5, Math.floor(i / 325) % 4, Math.floor(i / 1300)),
);

export const cases: Uint8Array[] = [...loops, ...durations, ...ints(), ...utf8, ...floats, ...formats, ...parsed, ...singlesCases, ...singlesFormats, ...singlesParsed, ...dyns, ...cells, ...shared, ...locks, ...onces, ...bounds, ...unsafeCells, ...edits, ...subslices, ...consumers, ...closures, ...elements, ...ranges, ...reducers, ...sliceEdits, ...drains, ...iters, ...hinted, ...hints, ...reserves];
