// `f64`, for rust2-ts output: a number, which is the same IEEE 754 double, so its
// operators are JS's. These are its methods where JS differs from std or has none, each
// its own function, `f64Round`, so a bundler drops every one translated code never
// calls. `rt.ts` re-exports all of it.
//
// JS doesn't specify which NaN an operation makes, so a NaN's payload and sign are the
// engine's: `to_bits` reads what it keeps.

import { Err, type FmtResult, type Formatter, fmtPadFormatted, fmtToString, Ok, panic, type Result } from "../rt.ts";
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

/** `f64::clamp`, which panics with both bounds' `Debug` unless `min <= max`. */
export function f64Clamp(x: f64, min: f64, max: f64): f64 {
	if (!(min <= max)) {
		const debug = (v: f64) => fmtToString((f) => f64Debug(v, f));
		panic(`min > max, or either was NaN. min = ${debug(min)}, max = ${debug(max)}`);
	}
	return x < min ? min : x > max ? max : x;
}

// Formatting, as `core::fmt::float` does it. A finite nonzero magnitude is digits and the
// power of ten that `0.digits` scales by, as `flt2dec` makes them.

/** `x`'s shortest digits that read back as it, the closest to it of those, and the greater of two as close. */
function shortest(x: number): [string, number] {
	// JS picks the shortest and then the closest too, but the even one of two as close.
	const [m, e] = x.toExponential().split("e");
	const digits = m.replace(".", "");
	const exp = Number(e) + 1;
	// Two are as close only where `x` ends half a digit past them: one more binary digit
	// past the point than they have decimal ones.
	const places = digits.length - exp + 1;
	if (places < 1 || fractionBits(x) !== places) return [digits, exp];
	const [all, at] = exact(x);
	// The greater ending in 0 would be shorter, so it can't read back as `x`.
	if (all !== digits + "5" || at !== exp || digits.endsWith("9")) return [digits, exp];
	const up = digits.slice(0, -1) + String.fromCharCode(digits.charCodeAt(digits.length - 1) + 1);
	return nearest(up, exp - up.length) === x ? [up, exp] : [digits, exp];
}

/** How many binary digits `x`, finite and nonzero, has past the point. */
function fractionBits(x: number): number {
	const view = bytes();
	view.setFloat64(0, x);
	const hi = view.getUint32(0);
	const lo = view.getUint32(4);
	const biased = (hi >>> 20) & 0x7ff;
	// The significand's lowest set bit, in its low word or else its high one.
	const word = lo !== 0 ? lo : (hi & 0xfffff) | (biased === 0 ? 0 : 0x100000);
	const zeros = 31 - Math.clz32(word & -word) + (lo !== 0 ? 0 : 32);
	return Math.max(0, 1075 - Math.max(biased, 1) - zeros);
}

/** `x`'s exact digits, without trailing zeros, which a double always has: its significand times a power of two. */
function exact(x: number): [string, number] {
	const view = bytes();
	view.setFloat64(0, x);
	const bits = view.getBigUint64(0);
	const biased = Number(bits >> 52n);
	const significand = biased === 0 ? bits : (bits & 0xf_ffff_ffff_ffffn) | 0x10_0000_0000_0000n;
	const e = Math.max(biased, 1) - 1075;
	// m * 2^-k is m * 5^k / 10^k.
	const all = (e >= 0 ? significand << BigInt(e) : significand * 5n ** BigInt(-e)).toString();
	return [all.replace(/0+$/, ""), all.length + Math.min(e, 0)];
}

/** The first `n` of `digits`, rounded half to even on the rest, as `format_exact` does; none if they round to zero. */
function round(digits: string, exp: number, n: number): [string, number] {
	if (n >= digits.length) return [digits, exp];
	if (n < 0) return ["", exp];
	const kept = digits.slice(0, n);
	const next = digits[n];
	const half = next === "5" && /^0*$/.test(digits.slice(n + 1));
	// A tie with no digit kept rounds to the zero before it, which is even.
	const odd = n > 0 && kept.charCodeAt(n - 1) % 2 === 1;
	if (next < "5" || (half && !odd)) return [kept, exp];
	const nines = kept.search(/9*$/);
	if (nines === 0) return ["1".padEnd(Math.max(n, 1), "0"), exp + 1];
	const carried = String.fromCharCode(kept.charCodeAt(nines - 1) + 1);
	return [kept.slice(0, nines - 1) + carried + "0".repeat(n - nines), exp];
}

/** `digits_to_dec_str`: at least `frac` digits past the point. */
function decimal(digits: string, exp: number, frac: number): string {
	if (exp <= 0) return `0.${"0".repeat(-exp)}${digits}`.padEnd(2 + Math.max(frac, digits.length - exp), "0");
	if (exp < digits.length) return `${digits.slice(0, exp)}.${digits.slice(exp)}`.padEnd(exp + 1 + Math.max(frac, digits.length - exp), "0");
	return digits.padEnd(exp, "0") + (frac > 0 ? `.${"0".repeat(frac)}` : "");
}

/** `digits_to_exp_str`: at least `frac` digits past the point. */
function exponential(digits: string, exp: number, frac: number, e: "e" | "E"): string {
	const rest = digits.slice(1).padEnd(frac, "0");
	return `${digits[0]}${rest === "" ? "" : `.${rest}`}${e}${exp - 1}`;
}

/** `0`, with `frac` zeros past the point. */
const zero = (frac: number): string => (frac > 0 ? `0.${"0".repeat(frac)}` : "0");

/** `to_shortest_str`: the shortest digits, with at least `frac` past the point. */
function shortestFixed(x: number, frac: number): string {
	if (x === 0) return zero(frac);
	const [digits, exp] = shortest(x);
	return decimal(digits, exp, frac);
}

/** `to_exact_fixed_str`: rounded to `frac` digits past the point. */
function exactFixed(x: number, frac: number): string {
	if (x === 0) return zero(frac);
	const [all, at] = exact(x);
	const [digits, exp] = round(all, at, at + frac);
	return digits === "" ? zero(frac) : decimal(digits, exp, frac);
}

/** `to_shortest_exp_str` without a precision, else `to_exact_exp_str`: `precision` digits past the point. */
function scientific(x: number, precision: number | undefined, e: "e" | "E"): string {
	if (x === 0) return `${zero(precision ?? 0)}${e}0`;
	if (precision === undefined) {
		const [digits, exp] = shortest(x);
		return exponential(digits, exp, 0, e);
	}
	const [all, at] = exact(x);
	const [digits, exp] = round(all, at, precision + 1);
	return exponential(digits, exp, precision, e);
}

/** Write `x` padded, its magnitude by `finite`: a NaN has no sign, and -0.0 has one. */
function write(x: f64, f: Formatter, finite: (magnitude: number) => string): FmtResult {
	if (x !== x) return fmtPadFormatted(f, "", "NaN");
	const sign = f64IsSignNegative(x) ? "-" : f.spec.plus ? "+" : "";
	const magnitude = Math.abs(x);
	return fmtPadFormatted(f, sign, magnitude === Infinity ? "inf" : finite(magnitude));
}

/** `f64`'s `Display`: the shortest digits, or rounded to the precision. */
export function f64Display(x: f64, f: Formatter): FmtResult {
	const precision = f.spec.precision;
	return write(x, f, (m) => (precision === undefined ? shortestFixed(m, 0) : exactFixed(m, precision)));
}

/** `f64`'s `Debug`: as `Display`, but with a point, and an exponent below 1e-4 and from 1e16. */
export function f64Debug(x: f64, f: Formatter): FmtResult {
	const precision = f.spec.precision;
	return write(x, f, (m) => {
		if (precision !== undefined) return exactFixed(m, precision);
		return (m !== 0 && m < 1e-4) || m >= 1e16 ? scientific(m, undefined, "e") : shortestFixed(m, 1);
	});
}

/** `f64`'s `LowerExp`. */
export const f64LowerExp = (x: f64, f: Formatter): FmtResult => write(x, f, (m) => scientific(m, f.spec.precision, "e"));

/** `f64`'s `UpperExp`. */
export const f64UpperExp = (x: f64, f: Formatter): FmtResult => write(x, f, (m) => scientific(m, f.spec.precision, "E"));

// Parsing, as `core::num::dec2flt` does it.

/** `core::num::ParseFloatError`: whether the string was empty or not a float. */
export type ParseFloatError = { readonly kind: "Empty" | "Invalid" };

/** Every power of ten a double holds exactly. */
const POWERS = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22];

/** `str::parse::<f64>`: std's grammar, without JS's whitespace, hex, or `Infinity`, rounded to the nearest double, half to even. */
export function f64FromStr(s: string): Result<f64, ParseFloatError> {
	if (s === "") return Err({ kind: "Empty" });
	const signed = s[0] === "-" || s[0] === "+";
	const rest = signed ? s.slice(1) : s;
	const number = /^([0-9]*)(?:\.([0-9]*))?(?:[eE]([+-]?)([0-9]+))?$/.exec(rest);
	let x: number;
	if (number !== null && number[1].length + (number[2]?.length ?? 0) > 0) {
		const [, int, frac = "", sign, digits = ""] = number;
		// std stops adding exponent digits once past 0x10000.
		let exp = 0;
		for (let i = 0; i < digits.length; i++) if (exp < 0x10000) exp = exp * 10 + digits.charCodeAt(i) - 48;
		x = nearest(int + frac, (sign === "-" ? -exp : exp) - frac.length);
	} else if (/^(?:inf|infinity)$/i.test(rest)) {
		x = Infinity;
	} else if (/^nan$/i.test(rest)) {
		x = NaN;
	} else {
		return Err({ kind: "Invalid" });
	}
	return Ok(s[0] === "-" ? -x : x);
}

/** The double nearest `digits * 10^exp`, half to even. */
function nearest(digits: string, exp: number): number {
	const trimmed = digits.replace(/^0+/, "").replace(/0+$/, "");
	if (trimmed === "") return 0;
	exp += digits.length - digits.replace(/0+$/, "").length;
	const n = trimmed.length;
	// Both operands exact, so the one operation rounds once, as Clinger's fast path does.
	if (n <= 15 && Math.abs(exp) <= 22) return exp < 0 ? Number(trimmed) / POWERS[-exp] : Number(trimmed) * POWERS[exp];
	// At least 10^309, or less than half the least subnormal.
	if (n + exp > 310) return Infinity;
	if (n + exp < -324) return 0;
	const big = BigInt(trimmed);
	return exp < 0 ? quotient(big, 10n ** BigInt(-exp)) : quotient(big * 10n ** BigInt(exp), 1n);
}

/** The double nearest `num / den`, half to even, built from its bits so nothing rounds twice. */
function quotient(num: bigint, den: bigint): number {
	const TWO52 = 1n << 52n;
	// The power of two that leaves a 53-bit significand, or the subnormals' one.
	let e = Math.max(num.toString(2).length - den.toString(2).length - 53, -1074);
	const divide = (): [bigint, bigint, bigint] => {
		const [n, d] = e < 0 ? [num << BigInt(-e), den] : [num, den << BigInt(e)];
		return [n / d, n % d, d];
	};
	let [q, r, d] = divide();
	if (q >= TWO52 << 1n) {
		e++;
		[q, r, d] = divide();
	}
	if (2n * r > d || (2n * r === d && (q & 1n) === 1n)) q++;
	if (q === TWO52 << 1n) {
		q = TWO52;
		e++;
	}
	if (e + 1075 >= 2047) return Infinity;
	const view = bytes();
	view.setBigUint64(0, q < TWO52 ? q : (BigInt(e + 1075) << 52n) | (q - TWO52));
	return view.getFloat64(0);
}
