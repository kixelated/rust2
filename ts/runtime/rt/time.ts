// `core::time::Duration`, for rust2-ts output: std's whole seconds and nanoseconds, with
// each method its own function, `durationAdd`, so a bundler drops every one translated
// code never calls. `rt.ts` re-exports all of it.
//
// A `Duration` never changes once made, so values share it. Its arithmetic is std's: a
// sum past `u64::MAX` seconds is `None`, saturates, or panics with std's message.

import { type FmtResult, type Formatter, fmtPad, panic } from "../rt.ts";
import { cmp, type Hasher, hashInt, type Ordering } from "./collections.ts";
import { type u32, type u64, type u128, u64CheckedAdd, u64CheckedMul, u64CheckedSub, u64Div, u64Eq, u64Rem, u128Add, u128Cast, u128Mul } from "./int.ts";

/** `core::time::Duration`: whole seconds, and the nanoseconds past them, below 10^9. */
export interface Duration {
	readonly secs: u64;
	readonly nanos: u32;
}

const NANOS_PER_SEC = 1_000_000_000;
const MAX_SAFE = 0x1fffffffffffff;

/** `Duration::MAX`, whose seconds are `u64::MAX`'s limbs. A function, as a bundler keeps a constant object. */
function max(): Duration {
	return { secs: { hi: 0xffffffff, lo: 0xffffffff } as u64, nanos: NANOS_PER_SEC - 1 };
}

/** `Duration::new`: `nanos` past a second carries into `secs`, panicking past `u64::MAX`. */
export function durationNew(secs: u64, nanos: u32): Duration {
	if (nanos < NANOS_PER_SEC) return { secs, nanos };
	const carried = u64CheckedAdd(secs, Math.floor(nanos / NANOS_PER_SEC));
	if (carried === undefined) panic("adding carry-over seconds from nanoseconds should not exceed u64::MAX");
	return { secs: carried, nanos: nanos % NANOS_PER_SEC };
}

/** `Duration::from_secs`. */
export function durationFromSecs(secs: u64): Duration {
	return { secs, nanos: 0 };
}

/** `Duration::from_millis`. */
export function durationFromMillis(millis: u64): Duration {
	return { secs: u64Div(millis, 1000), nanos: (u64Rem(millis, 1000) as number) * 1_000_000 };
}

/** `Duration::from_micros`. */
export function durationFromMicros(micros: u64): Duration {
	return { secs: u64Div(micros, 1_000_000), nanos: (u64Rem(micros, 1_000_000) as number) * 1000 };
}

/** `Duration::from_nanos`. */
export function durationFromNanos(nanos: u64): Duration {
	return { secs: u64Div(nanos, NANOS_PER_SEC), nanos: u64Rem(nanos, NANOS_PER_SEC) as number };
}

/** `Duration::is_zero`. */
export function durationIsZero(d: Duration): boolean {
	return d.secs === 0 && d.nanos === 0;
}

/** `Duration::as_secs`: the whole seconds. */
export function durationAsSecs(d: Duration): u64 {
	return d.secs;
}

/** `Duration::subsec_millis`. */
export function durationSubsecMillis(d: Duration): u32 {
	return Math.floor(d.nanos / 1_000_000);
}

/** `Duration::subsec_micros`. */
export function durationSubsecMicros(d: Duration): u32 {
	return Math.floor(d.nanos / 1000);
}

/** `Duration::subsec_nanos`. */
export function durationSubsecNanos(d: Duration): u32 {
	return d.nanos;
}

/** `secs * per + sub` as a `u128`, a number while that is a safe integer. */
function total(secs: u64, per: number, sub: number): u128 {
	if (typeof secs === "number") {
		// Exact up to 2^53, and past it rounded to no less than 2^53.
		const n = secs * per + sub;
		if (n <= MAX_SAFE) return n;
	}
	return u128Add(u128Mul(u128Cast(secs), per), sub);
}

/** `Duration::as_millis`. */
export function durationAsMillis(d: Duration): u128 {
	return total(d.secs, 1000, Math.floor(d.nanos / 1_000_000));
}

/** `Duration::as_micros`. */
export function durationAsMicros(d: Duration): u128 {
	return total(d.secs, 1_000_000, Math.floor(d.nanos / 1000));
}

/** `Duration::as_nanos`. */
export function durationAsNanos(d: Duration): u128 {
	return total(d.secs, NANOS_PER_SEC, d.nanos);
}

/** `Duration::checked_add`: `a + b`, or `undefined` past `u64::MAX` seconds. */
export function durationCheckedAdd(a: Duration, b: Duration): Duration | undefined {
	let secs = u64CheckedAdd(a.secs, b.secs);
	if (secs === undefined) return undefined;
	let nanos = a.nanos + b.nanos;
	if (nanos >= NANOS_PER_SEC) {
		nanos -= NANOS_PER_SEC;
		secs = u64CheckedAdd(secs, 1);
		if (secs === undefined) return undefined;
	}
	return { secs, nanos };
}

/** `Duration::checked_sub`: `a - b`, or `undefined` below zero. */
export function durationCheckedSub(a: Duration, b: Duration): Duration | undefined {
	let secs = u64CheckedSub(a.secs, b.secs);
	if (secs === undefined) return undefined;
	if (a.nanos >= b.nanos) return { secs, nanos: a.nanos - b.nanos };
	secs = u64CheckedSub(secs, 1);
	if (secs === undefined) return undefined;
	return { secs, nanos: a.nanos + NANOS_PER_SEC - b.nanos };
}

/** `Duration::checked_mul` by a `u32`: `d * n`, or `undefined` past `u64::MAX` seconds. */
export function durationCheckedMul(d: Duration, n: u32): Duration | undefined {
	// The nanoseconds' product is below 2^62, past a safe integer only for large factors.
	let carry: number;
	let nanos: number;
	const product = d.nanos * n;
	if (product <= MAX_SAFE) {
		carry = Math.floor(product / NANOS_PER_SEC);
		nanos = product % NANOS_PER_SEC;
	} else {
		const exact = BigInt(d.nanos) * BigInt(n);
		carry = Number(exact / 1_000_000_000n);
		nanos = Number(exact % 1_000_000_000n);
	}
	const secs = u64CheckedMul(d.secs, n as u64);
	if (secs === undefined) return undefined;
	const carried = u64CheckedAdd(secs, carry);
	if (carried === undefined) return undefined;
	return { secs: carried, nanos };
}

/** `Duration::checked_div` by a `u32`: `d / n`, truncated, or `undefined` if `n` is zero. */
export function durationCheckedDiv(d: Duration, n: u32): Duration | undefined {
	if (n === 0) return undefined;
	const secs = u64Div(d.secs, n as u64);
	// What `secs` leaves is below `n`, so its nanoseconds are below 2^62.
	const left = u64Rem(d.secs, n as u64) as number;
	const rest = left * NANOS_PER_SEC + (d.nanos % n);
	const nanos = Math.floor(d.nanos / n) + (rest <= MAX_SAFE ? Math.floor(rest / n) : Number((BigInt(left) * 1_000_000_000n + BigInt(d.nanos % n)) / BigInt(n)));
	return { secs, nanos };
}

/** `Duration::saturating_add`: `a + b`, or `Duration::MAX` past it. */
export function durationSaturatingAdd(a: Duration, b: Duration): Duration {
	return durationCheckedAdd(a, b) ?? max();
}

/** `Duration::saturating_sub`: `a - b`, or zero below it. */
export function durationSaturatingSub(a: Duration, b: Duration): Duration {
	return durationCheckedSub(a, b) ?? { secs: 0, nanos: 0 };
}

/** `Duration::saturating_mul` by a `u32`: `d * n`, or `Duration::MAX` past it. */
export function durationSaturatingMul(d: Duration, n: u32): Duration {
	return durationCheckedMul(d, n) ?? max();
}

/** `Duration::abs_diff`: the distance between `a` and `b`. */
export function durationAbsDiff(a: Duration, b: Duration): Duration {
	return durationCheckedSub(a, b) ?? (durationCheckedSub(b, a) as Duration);
}

/** `a + b`, panicking past `u64::MAX` seconds. */
export function durationAdd(a: Duration, b: Duration): Duration {
	return durationCheckedAdd(a, b) ?? panic("adding durations should not exceed u64::MAX");
}

/** `a - b`, panicking below zero. */
export function durationSub(a: Duration, b: Duration): Duration {
	return durationCheckedSub(a, b) ?? panic("subtracting durations should not be negative");
}

/** `d * n` by a `u32`, panicking past `u64::MAX` seconds. */
export function durationMul(d: Duration, n: u32): Duration {
	return durationCheckedMul(d, n) ?? panic("multiplying duration by scalar should not exceed u64::MAX");
}

/** `n * d`, which std computes as `d * n`. */
export function durationTimes(n: u32, d: Duration): Duration {
	return durationMul(d, n);
}

/** `d / n` by a `u32`, truncated, panicking if `n` is zero. */
export function durationDiv(d: Duration, n: u32): Duration {
	return durationCheckedDiv(d, n) ?? panic("divisor should not be zero");
}

/** `PartialEq::eq`. */
export function durationEq(a: Duration, b: Duration): boolean {
	return a.nanos === b.nanos && u64Eq(a.secs, b.secs);
}

/** `Ord::cmp`: by seconds, then nanoseconds. */
export function durationCmp(a: Duration, b: Duration): Ordering {
	return cmp(a.secs, b.secs) || cmp(a.nanos, b.nanos);
}

/** `Hash::hash`: its seconds, then its nanoseconds, as the derive hashes them. */
export function durationHash(d: Duration, s: Hasher): void {
	hashInt(d.secs, s);
	hashInt(d.nanos, s);
}

/** `Debug`, as std writes it: `1.5s`, `20ms`, `3µs`, or `0ns`, padded to the width. */
export function durationDebug(d: Duration, f: Formatter): FmtResult {
	const prefix = f.spec.plus ? "+" : "";
	if (d.secs !== 0) return decimal(f, d.secs, d.nanos, NANOS_PER_SEC / 10, prefix, "s");
	if (d.nanos >= 1_000_000) return decimal(f, Math.floor(d.nanos / 1_000_000), d.nanos % 1_000_000, 100_000, prefix, "ms");
	if (d.nanos >= 1000) return decimal(f, Math.floor(d.nanos / 1000), d.nanos % 1000, 100, prefix, "µs");
	return decimal(f, d.nanos as u64, 0, 1, prefix, "ns");
}

/**
 * std's `fmt_decimal`: `whole`, then the digits of `fraction` from `divisor` down, rounded
 * half to even to the precision or else up to the last nonzero one.
 */
function decimal(f: Formatter, whole: u64, fraction: number, divisor: number, prefix: string, unit: string): FmtResult {
	const precision = f.spec.precision;
	let digits = "";
	while (fraction > 0 && digits.length < (precision ?? 9)) {
		digits += Math.floor(fraction / divisor);
		fraction %= divisor;
		divisor /= 10;
	}
	let carry = false;
	if (fraction > 0 && fraction >= divisor * 5) {
		const last = digits.length > 0 ? Number(digits[digits.length - 1]) : typeof whole === "number" ? whole : whole.lo;
		if (fraction > divisor * 5 || last % 2 === 1) {
			const nines = /9*$/.exec(digits)![0].length;
			const at = digits.length - nines - 1;
			carry = at < 0;
			digits = (carry ? "" : digits.slice(0, at) + (Number(digits[at]) + 1)) + "0".repeat(nines);
		}
	}
	const big = typeof whole === "number" ? BigInt(whole) : (BigInt(whole.hi) << 32n) | BigInt(whole.lo);
	const int = carry ? String(big + 1n) : String(big);
	const point = Math.min(precision ?? digits.length, 9) > 0 ? `.${digits.padEnd(precision ?? digits.length, "0")}` : "";
	// A width pads the whole text, which the precision has already cut.
	const uncut = precision === undefined ? f : { out: f.out, spec: { ...f.spec, precision: undefined } };
	return fmtPad(uncut, `${prefix}${int}${point}${unit}`);
}
