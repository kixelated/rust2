// `f64`, for rust2-ts output: a number, which is the same IEEE 754 double, so its
// operators are JS's. These are its methods where JS differs from std or has none, each
// its own function, `f64Round`, so a bundler drops every one translated code never
// calls. `rt.ts` re-exports all of it.
//
// JS doesn't specify which NaN an operation makes, so a NaN's payload and sign are the
// engine's: `to_bits` reads what it keeps.

import type { Ordering } from "./collections.ts";
import { type f64, type u64, u64FromHalves } from "./int.ts";

/** Eight bytes that hold a double while its bits are read or written, made on first use so a bundler drops them unused. */
let scratch: DataView | undefined;
const bytes = (): DataView => (scratch ??= new DataView(new ArrayBuffer(8)));
const TWO32 = 4294967296;
const TWO52 = 4503599627370496;
/** `f64::MIN_POSITIVE`, the least normal double. */
const MIN_POSITIVE = 2.2250738585072014e-308;

/** `f64::round`: to the nearest integer, half away from zero. */
export function f64Round(x: f64): f64 {
	const t = Math.trunc(x);
	// `x - t` is exact, and NaN for an infinity, which `t` already is.
	return Math.abs(x - t) >= 0.5 ? t + Math.sign(x) : t;
}

/** `f64::round_ties_even`: to the nearest integer, half to even. */
export function f64RoundTiesEven(x: f64): f64 {
	const a = Math.abs(x);
	// Every double from 2^52 up is an integer; below it, adding 2^52 rounds to one, ties to even.
	if (!(a < TWO52)) return x;
	const r = a + TWO52 - TWO52;
	return x < 0 ? -r : x === 0 ? x : r;
}

/** `f64::fract`: `x - x.trunc()`. */
export const f64Fract = (x: f64): f64 => x - Math.trunc(x);

/** `f64::is_sign_negative`: whether the sign bit is set, as it is for -0.0. */
export function f64IsSignNegative(x: f64): boolean {
	if (x === x) return x < 0 || 1 / x < 0;
	const view = bytes();
	view.setFloat64(0, x);
	return view.getUint8(0) >= 0x80;
}

/** `f64::is_sign_positive`: whether the sign bit is clear. */
export const f64IsSignPositive = (x: f64): boolean => !f64IsSignNegative(x);

/** `f64::signum`: 1.0 or -1.0 by the sign bit, so -1.0 for -0.0, and NaN for NaN. */
export const f64Signum = (x: f64): f64 => (x !== x ? NaN : f64IsSignNegative(x) ? -1 : 1);

/** `f64::copysign`: `x`'s magnitude with `y`'s sign bit. */
export function f64Copysign(x: f64, y: f64): f64 {
	const negative = f64IsSignNegative(y);
	if (x === x) return negative ? -Math.abs(x) : Math.abs(x);
	const view = bytes();
	view.setFloat64(0, x);
	const top = view.getUint8(0) & 0x7f;
	view.setUint8(0, negative ? top | 0x80 : top);
	return view.getFloat64(0);
}

/** `f64::is_infinite`. */
export const f64IsInfinite = (x: f64): boolean => Math.abs(x) === Infinity;

/** `f64::is_normal`: neither zero, subnormal, infinite, nor NaN. */
export function f64IsNormal(x: f64): boolean {
	const a = Math.abs(x);
	return a >= MIN_POSITIVE && a < Infinity;
}

/** `f64::is_subnormal`. */
export function f64IsSubnormal(x: f64): boolean {
	const a = Math.abs(x);
	return a > 0 && a < MIN_POSITIVE;
}

/** `x`'s bits moved by one toward +infinity if `up`, from a finite or infinite non-NaN. */
function step(x: number, up: boolean): number {
	if (x === 0) return up ? 5e-324 : -5e-324;
	const view = bytes();
	view.setFloat64(0, x);
	let hi = view.getUint32(0);
	let lo = view.getUint32(4);
	// The magnitude grows if it moves away from zero.
	if (up === x > 0) {
		lo = (lo + 1) >>> 0;
		if (lo === 0) hi++;
	} else {
		if (lo === 0) hi--;
		lo = (lo - 1) >>> 0;
	}
	view.setUint32(0, hi);
	view.setUint32(4, lo);
	return view.getFloat64(0);
}

/** `f64::next_up`: the least double greater than `x`. */
export const f64NextUp = (x: f64): f64 => (x !== x || x === Infinity ? x : step(x, true));

/** `f64::next_down`: the greatest double less than `x`. */
export const f64NextDown = (x: f64): f64 => (x !== x || x === -Infinity ? x : step(x, false));

/** `f64::max`: the greater, ignoring a NaN. */
export const f64Max = (a: f64, b: f64): f64 => (a !== a ? b : b !== b ? a : Math.max(a, b));

/** `f64::min`: the lesser, ignoring a NaN. */
export const f64Min = (a: f64, b: f64): f64 => (a !== a ? b : b !== b ? a : Math.min(a, b));

/** `f64::midpoint`: halved before the sum where the sum could overflow. */
export function f64Midpoint(a: f64, b: f64): f64 {
	const hi = Number.MAX_VALUE * 0.5;
	return Math.abs(a) <= hi && Math.abs(b) <= hi ? (a + b) * 0.5 : a * 0.5 + b * 0.5;
}

/** `f64::to_bits`. */
export function f64ToBits(x: f64): u64 {
	const view = bytes();
	view.setFloat64(0, x);
	return u64FromHalves(view.getUint32(0), view.getUint32(4));
}

/** `f64::from_bits`. */
export function f64FromBits(v: u64): f64 {
	const view = bytes();
	view.setUint32(0, typeof v === "number" ? Math.floor(v / TWO32) : v.hi);
	view.setUint32(4, typeof v === "number" ? v >>> 0 : v.lo);
	return view.getFloat64(0);
}

function toBytes(x: number, little: boolean): Uint8Array {
	const out = new Uint8Array(8);
	new DataView(out.buffer).setFloat64(0, x, little);
	return out;
}

function fromBytes(bytes: Uint8Array, little: boolean): number {
	return new DataView(bytes.buffer, bytes.byteOffset, 8).getFloat64(0, little);
}

/** `f64::to_be_bytes`. */
export const f64ToBeBytes = (x: f64): Uint8Array => toBytes(x, false);

/** `f64::to_le_bytes`, and `to_ne_bytes`, as wasm32 is little-endian. */
export const f64ToLeBytes = (x: f64): Uint8Array => toBytes(x, true);

/** `f64::from_be_bytes`. */
export const f64FromBeBytes = (bytes: Uint8Array): f64 => fromBytes(bytes, false);

/** `f64::from_le_bytes`, and `from_ne_bytes`, as wasm32 is little-endian. */
export const f64FromLeBytes = (bytes: Uint8Array): f64 => fromBytes(bytes, true);

/** `f64::total_cmp`: IEEE 754's total order, by the bits, with a negative's flipped. */
export function f64TotalCmp(a: f64, b: f64): Ordering {
	const view = bytes();
	view.setFloat64(0, a);
	let ah = view.getInt32(0);
	let al = view.getUint32(4);
	if (ah < 0) {
		ah ^= 0x7fffffff;
		al = ~al >>> 0;
	}
	view.setFloat64(0, b);
	let bh = view.getInt32(0);
	let bl = view.getUint32(4);
	if (bh < 0) {
		bh ^= 0x7fffffff;
		bl = ~bl >>> 0;
	}
	return ah < bh ? -1 : ah > bh ? 1 : al < bl ? -1 : al > bl ? 1 : 0;
}

/** `f64`'s `PartialOrd::partial_cmp`: `undefined`, which is `None`, if either is NaN. */
export const f64PartialCmp = (a: f64, b: f64): Ordering | undefined => (a < b ? -1 : a > b ? 1 : a === b ? 0 : undefined);

/** `f64::powi`, multiplied by squaring as compiler-builtins' `__powidf2` is. */
export function f64Powi(x: f64, n: number): f64 {
	let a = x;
	let pow = Math.abs(n);
	let mul = 1;
	for (;;) {
		if (pow & 1) mul *= a;
		// `>>>` keeps 2^31, the magnitude of `i32::MIN`, unsigned.
		pow >>>= 1;
		if (pow === 0) break;
		a *= a;
	}
	return n < 0 ? 1 / mul : mul;
}

/** `f64::powf`, which is libm's `pow`: 1 for a base of 1, and for -1 to an infinite power, where `Math.pow` is NaN. */
export const f64Powf = (x: f64, y: f64): f64 => (x === 1 || (x === -1 && Math.abs(y) === Infinity) ? 1 : Math.pow(x, y));

/** `f64::sin_cos`. */
export const f64SinCos = (x: f64): [f64, f64] => [Math.sin(x), Math.cos(x)];

/** `f64::atanh`, as std computes it rather than libm. */
export const f64Atanh = (x: f64): f64 => 0.5 * Math.log1p((2 * x) / (1 - x));

/** `f64::div_euclid`. */
export function f64DivEuclid(x: f64, rhs: f64): f64 {
	const q = Math.trunc(x / rhs);
	if (x % rhs < 0) return rhs > 0 ? q - 1 : q + 1;
	return q;
}

/** `f64::rem_euclid`. */
export function f64RemEuclid(x: f64, rhs: f64): f64 {
	const r = x % rhs;
	return r < 0 ? r + Math.abs(rhs) : r;
}

/** `f64`'s `+`, which `Iterator::sum` folds with. */
export const f64Add = (a: f64, b: f64): f64 => a + b;

/** `f64`'s `*`, which `Iterator::product` folds with. */
export const f64Mul = (a: f64, b: f64): f64 => a * b;
