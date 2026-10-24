// The runtime of rust2-ts output: what Rust's std does that plain JS does not.
//
// Integers are numbers, but a 64- or 128-bit one past 2^53 is an immutable object of
// 32-bit limbs. Arithmetic is checked as a debug build checks it: overflow panics, it
// never wraps silently. Every panic's message is the one native Rust panics with.

export * from "./rt/iter.ts";

/** A Rust panic. */
export class Panic extends Error {
	override name = "Panic";
}

/**
 * Panic with `message`. A formatted message keeps what was written before a value
 * failed to format, as std's does.
 */
export function panic(message: string | Arguments): never {
	if (typeof message === "string") throw new Panic(message);
	const out = new StringWriter();
	write(out, message);
	throw new Panic(out.s);
}

/** A `match` without arms, on a value Rust proved can't exist, such as a `(Void, u8)`. */
export function unreachable(_value: unknown): never {
	throw new Panic("internal error: entered unreachable code");
}

/** `assert_eq!` or `assert_ne!` failing: `op` is `==` or `!=`. */
export function assertFailed(op: string, left: Fmt, right: Fmt, args?: Arguments): never {
	const failed: Arguments = args === undefined ? [" failed"] : [" failed: ", (f) => f.writeFmt(args)];
	panic(["assertion `left ", op, " right`", ...failed, "\n  left: ", left, "\n right: ", right]);
}

// Integers

declare const brand: unique symbol;

/** A `u64` past 2^53: its high and low 32 bits, each a uint32. */
export type U64Limbs = { readonly hi: number; readonly lo: number; readonly [brand]: "u64" };
/** An `i64` past 2^53 in magnitude: its high 32 bits as an int32, and its low as a uint32. */
export type I64Limbs = { readonly hi: number; readonly lo: number; readonly [brand]: "i64" };
/** A `u128` past 2^53: four 32-bit limbs, `hi`, `w2`, `w1`, and `lo`, each a uint32. */
export type U128Limbs = { readonly hi: number; readonly w2: number; readonly w1: number; readonly lo: number; readonly [brand]: "u128" };
/** An `i128` past 2^53 in magnitude: as a `u128`'s, but `hi` is an int32. */
export type I128Limbs = { readonly hi: number; readonly w2: number; readonly w1: number; readonly lo: number; readonly [brand]: "i128" };
/** Limbs, whose value is theirs with `hi` read in the type's sign. */
export type Limbs = U64Limbs | I64Limbs | U128Limbs | I128Limbs;

/**
 * A `u64`: a number while it is a safe integer, at most 2^53 - 1, and its limbs past
 * that. Each value has one form, so two numbers compare with `===`, and a number never
 * equals limbs.
 */
export type U64 = number | U64Limbs;
/** An `i64`: a number while it is a safe integer, and its limbs past that. */
export type I64 = number | I64Limbs;
/** A `u128`: a number while it is a safe integer, and its limbs past that. */
export type U128 = number | U128Limbs;
/** An `i128`: a number while it is a safe integer, and its limbs past that. */
export type I128 = number | I128Limbs;

/** The checks and methods of one integer type. */
export interface Int<T> {
	readonly MIN: T;
	readonly MAX: T;
	readonly BITS: number;
	add(a: T, b: T): T;
	sub(a: T, b: T): T;
	mul(a: T, b: T): T;
	neg(a: T): T;
	div(a: T, b: T): T;
	rem(a: T, b: T): T;
	shl(a: T, b: number | Limbs): T;
	shr(a: T, b: number | Limbs): T;
	checkedAdd(a: T, b: T): T | undefined;
	checkedSub(a: T, b: T): T | undefined;
	checkedMul(a: T, b: T): T | undefined;
	saturatingAdd(a: T, b: T): T;
	saturatingSub(a: T, b: T): T;
	wrappingAdd(a: T, b: T): T;
	wrappingSub(a: T, b: T): T;
	leadingOnes(a: T): number;
	leadingZeros(a: T): number;
	fromBeBytes(bytes: Uint8Array): T;
	toBeBytes(a: T): Uint8Array;
	tryFrom(value: number | Limbs): Result<T, TryFromIntError>;
	/** `Display`, and `Debug`, which writes the same. */
	display(a: T, f: Formatter): FmtResult;
	lowerHex(a: T, f: Formatter): FmtResult;
	upperHex(a: T, f: Formatter): FmtResult;
}

/** A 64- or 128-bit integer type: what JS operators do for numbers, as methods. */
export interface WideInt<T extends number | Limbs> extends Int<T> {
	/** A value from a safe integer, or from a bigint of any size, wrapped into range. */
	of(value: number | bigint): T;
	/** `value as T`. */
	cast(value: number | Limbs): T;
	/** The low 32 bits, as a uint32. */
	low(a: T): number;
	eq(a: T, b: T): boolean;
	ne(a: T, b: T): boolean;
	lt(a: T, b: T): boolean;
	le(a: T, b: T): boolean;
	gt(a: T, b: T): boolean;
	ge(a: T, b: T): boolean;
	max(a: T, b: T): T;
	min(a: T, b: T): T;
	not(a: T): T;
	and(a: T, b: T): T;
	or(a: T, b: T): T;
	xor(a: T, b: T): T;
}

const TWO32 = 4294967296;

/** A wide integer's value as a number: exact up to 2^53 in magnitude, rounded past it. */
function approx(v: Limbs): number {
	return "w1" in v ? ((v.hi * TWO32 + v.w2) * TWO32 + v.w1) * TWO32 + v.lo : v.hi * TWO32 + v.lo;
}

/** A wide integer's exact value. */
function exact(v: Limbs): bigint {
	if ("w1" in v) return (((((BigInt(v.hi) << 32n) + BigInt(v.w2)) << 32n) + BigInt(v.w1)) << 32n) + BigInt(v.lo);
	return (BigInt(v.hi) << 32n) + BigInt(v.lo);
}

function safe(n: number): boolean {
	return n <= Number.MAX_SAFE_INTEGER && n >= -Number.MAX_SAFE_INTEGER;
}

function shift(b: number | Limbs, bits: number, direction: string): number {
	const n = typeof b === "number" ? b : approx(b);
	if (n < 0 || n >= bits) panic(`attempt to shift ${direction} with overflow`);
	return n;
}

/**
 * An integer of 32 bits or less, in a number. Sums, differences, and quotients are
 * exact; a product is inexact only past 2^53, far outside every type's range, so its
 * check still fails. JS's 32-bit operators do the rest.
 */
function small(bits: number, signed: boolean): Int<number> {
	const min = signed ? -(2 ** (bits - 1)) : 0;
	const max = signed ? 2 ** (bits - 1) - 1 : 2 ** bits - 1;
	const mask = 2 ** bits - 1;
	// What the low `bits` bits read as.
	const wrap: (value: number) => number =
		bits === 32 ? (signed ? (v) => v | 0 : (v) => v >>> 0) : signed ? (v) => (v << (32 - bits)) >> (32 - bits) : (v) => v & mask;
	// The bits as unsigned, for leading zeros and hex.
	const unsigned = bits === 32 ? (v: number) => v >>> 0 : (v: number) => v & mask;
	const checked = (value: number, op: string): number => {
		if (value < min || value > max) panic(`attempt to ${op} with overflow`);
		return value;
	};
	const clamp = (value: number) => (value < min ? min : value > max ? max : value);
	const inRange = (value: number) => (value >= min && value <= max ? value : undefined);
	return {
		MIN: min,
		MAX: max,
		BITS: bits,
		add: (a, b) => checked(a + b, "add"),
		sub: (a, b) => checked(a - b, "subtract"),
		mul: (a, b) => checked(a * b, "multiply"),
		neg: (a) => checked(-a, "negate"),
		div(a, b) {
			if (b === 0) panic("attempt to divide by zero");
			return checked(Math.trunc(a / b), "divide");
		},
		rem(a, b) {
			if (b === 0) panic("attempt to calculate the remainder with a divisor of zero");
			if (signed && b === -1 && a === min) panic("attempt to calculate the remainder with overflow");
			return a % b;
		},
		shl: (a, b) => wrap(a << shift(b, bits, "left")),
		shr: (a, b) => (signed ? a >> shift(b, bits, "right") : a >>> shift(b, bits, "right")),
		checkedAdd: (a, b) => inRange(a + b),
		checkedSub: (a, b) => inRange(a - b),
		checkedMul: (a, b) => inRange(a * b),
		saturatingAdd: (a, b) => clamp(a + b),
		saturatingSub: (a, b) => clamp(a - b),
		wrappingAdd: (a, b) => wrap(a + b),
		wrappingSub: (a, b) => wrap(a - b),
		leadingOnes: (a) => Math.clz32(~(a << (32 - bits))),
		leadingZeros: (a) => Math.clz32(unsigned(a)) - (32 - bits),
		fromBeBytes(bytes) {
			let value = 0;
			for (let i = 0; i < bytes.length; i++) value = (value << 8) | bytes[i];
			return wrap(value);
		},
		toBeBytes(a) {
			const out = new Uint8Array(bits / 8);
			for (let i = out.length - 1; i >= 0; i--) {
				out[i] = a;
				a >>= 8;
			}
			return out;
		},
		tryFrom(value) {
			// Rounding past 2^53 keeps the order, so the bounds still test exactly.
			const n = typeof value === "number" ? value : approx(value);
			if (n >= min && n <= max) return Ok(n);
			return Err(new TryFromIntError(n < min ? "NegOverflow" : "PosOverflow"));
		},
		display: (a, f) => f.padIntegral(a >= 0, "", String(a < 0 ? -a : a)),
		lowerHex: (a, f) => f.padIntegral(true, "0x", unsigned(a).toString(16)),
		upperHex: (a, f) => f.padIntegral(true, "0x", unsigned(a).toString(16).toUpperCase()),
	};
}

/**
 * A wide integer type, exact through bigints: `box` wraps a value into the type's range,
 * and `compare` orders two of its objects.
 */
function wide<T extends Limbs>(bits: number, signed: boolean, box: (v: bigint) => T, compare: (a: T, b: T) => number): WideInt<T> {
	const min = signed ? -(1n << BigInt(bits - 1)) : 0n;
	const max = signed ? (1n << BigInt(bits - 1)) - 1n : (1n << BigInt(bits)) - 1n;
	const big = exact as (a: T) => bigint;
	const fits = (value: bigint) => value >= min && value <= max;
	const checked = (value: bigint, op: string): T => {
		if (!fits(value)) panic(`attempt to ${op} with overflow`);
		return box(value);
	};
	const clamp = (value: bigint) => box(value < min ? min : value > max ? max : value);
	const inRange = (value: bigint) => (fits(value) ? box(value) : undefined);
	const unsigned = (a: T) => BigInt.asUintN(bits, big(a));
	const leading = (value: bigint) => {
		let n = 0;
		for (let bit = BigInt(bits - 1); bit >= 0n && (value >> bit) & 1n; bit--) n++;
		return n;
	};
	const lt = (a: T, b: T) => compare(a, b) < 0;
	return {
		MIN: box(min),
		MAX: box(max),
		BITS: bits,
		add: (a, b) => checked(big(a) + big(b), "add"),
		sub: (a, b) => checked(big(a) - big(b), "subtract"),
		mul: (a, b) => checked(big(a) * big(b), "multiply"),
		neg: (a) => checked(-big(a), "negate"),
		div(a, b) {
			const d = big(b);
			if (d === 0n) panic("attempt to divide by zero");
			return checked(big(a) / d, "divide");
		},
		rem(a, b) {
			const d = big(b);
			if (d === 0n) panic("attempt to calculate the remainder with a divisor of zero");
			const n = big(a);
			if (signed && d === -1n && n === min) panic("attempt to calculate the remainder with overflow");
			return box(n % d);
		},
		shl: (a, b) => box(big(a) << BigInt(shift(b, bits, "left"))),
		shr: (a, b) => box((signed ? big(a) : unsigned(a)) >> BigInt(shift(b, bits, "right"))),
		checkedAdd: (a, b) => inRange(big(a) + big(b)),
		checkedSub: (a, b) => inRange(big(a) - big(b)),
		checkedMul: (a, b) => inRange(big(a) * big(b)),
		saturatingAdd: (a, b) => clamp(big(a) + big(b)),
		saturatingSub: (a, b) => clamp(big(a) - big(b)),
		wrappingAdd: (a, b) => box(big(a) + big(b)),
		wrappingSub: (a, b) => box(big(a) - big(b)),
		leadingOnes: (a) => leading(unsigned(a)),
		leadingZeros: (a) => leading(BigInt.asUintN(bits, ~big(a))),
		// A 32-bit word at a time: one bigint step per word, not per byte.
		fromBeBytes(bytes) {
			let value = 0n;
			for (let i = 0; i < bytes.length; i += 4) {
				const word = ((bytes[i] << 24) | (bytes[i + 1] << 16) | (bytes[i + 2] << 8) | bytes[i + 3]) >>> 0;
				value = (value << 32n) | BigInt(word);
			}
			return box(value);
		},
		toBeBytes(a) {
			const out = new Uint8Array(bits / 8);
			let value = unsigned(a);
			for (let i = out.length - 4; i >= 0; i -= 4) {
				const word = Number(BigInt.asUintN(32, value));
				out[i] = word >>> 24;
				out[i + 1] = word >>> 16;
				out[i + 2] = word >>> 8;
				out[i + 3] = word;
				value >>= 32n;
			}
			return out;
		},
		tryFrom(value) {
			const v = typeof value === "number" ? BigInt(value) : exact(value);
			if (fits(v)) return Ok(box(v));
			return Err(new TryFromIntError(v < min ? "NegOverflow" : "PosOverflow"));
		},
		display(a, f) {
			const v = big(a);
			return f.padIntegral(v >= 0n, "", (v < 0n ? -v : v).toString());
		},
		lowerHex: (a, f) => f.padIntegral(true, "0x", unsigned(a).toString(16)),
		upperHex: (a, f) => f.padIntegral(true, "0x", unsigned(a).toString(16).toUpperCase()),
		of: (value) => box(BigInt(value)),
		cast: (value) => box(typeof value === "number" ? BigInt(value) : exact(value)),
		low: (a) => a.lo,
		eq: (a, b) => compare(a, b) === 0,
		ne: (a, b) => compare(a, b) !== 0,
		lt,
		le: (a, b) => compare(a, b) <= 0,
		gt: (a, b) => compare(a, b) > 0,
		ge: (a, b) => compare(a, b) >= 0,
		max: (a, b) => (lt(b, a) ? a : b),
		min: (a, b) => (lt(b, a) ? b : a),
		not: (a) => box(~big(a)),
		and: (a, b) => box(big(a) & big(b)),
		or: (a, b) => box(big(a) | big(b)),
		xor: (a, b) => box(big(a) ^ big(b)),
	};
}

/**
 * `u64` or `i64`: exact through bigints, with the common operations computed on the
 * halves, which is several times faster.
 */
function wide64<T extends U64Limbs | I64Limbs>(signed: boolean): WideInt<T> {
	// The high half's range, and its value from any 32 bits.
	const hmin = signed ? -0x80000000 : 0;
	const hmax = signed ? 0x7fffffff : 0xffffffff;
	const high = signed ? (h: number) => h | 0 : (h: number) => h >>> 0;
	const make = (hi: number, lo: number) => ({ hi, lo }) as T;
	const box = (v: bigint) => make(Number(signed ? BigInt.asIntN(32, v >> 32n) : BigInt.asUintN(32, v >> 32n)), Number(BigInt.asUintN(32, v)));
	// A safe integer, wrapped into range.
	const fromNumber = (n: number) => make(high(Math.floor(n / TWO32)), n >>> 0);
	const compare = (a: T, b: T) => (a.hi !== b.hi ? (a.hi < b.hi ? -1 : 1) : a.lo === b.lo ? 0 : a.lo < b.lo ? -1 : 1);
	const lt = (a: T, b: T) => a.hi < b.hi || (a.hi === b.hi && a.lo < b.lo);
	const isZero = (a: T) => a.hi === 0 && a.lo === 0;
	const base = wide(64, signed, box, compare);

	// The high half of the last `mul32`.
	let mulHi = 0;
	/** The full product of two uint32s: its low half, with the high one in `mulHi`. */
	const mul32 = (a: number, b: number) => {
		const a0 = a & 0xffff;
		const a1 = a >>> 16;
		const b0 = b & 0xffff;
		const b1 = b >>> 16;
		const p00 = a0 * b0;
		const p01 = a0 * b1;
		const p10 = a1 * b0;
		const mid = (p00 >>> 16) + (p01 & 0xffff) + (p10 & 0xffff);
		mulHi = a1 * b1 + (p01 >>> 16) + (p10 >>> 16) + (mid >>> 16);
		return (((mid & 0xffff) << 16) | (p00 & 0xffff)) >>> 0;
	};

	return {
		...base,
		add(a, b) {
			const lo = a.lo + b.lo;
			const hi = a.hi + b.hi + (lo >= TWO32 ? 1 : 0);
			if (hi < hmin || hi > hmax) panic("attempt to add with overflow");
			return make(hi, lo >>> 0);
		},
		sub(a, b) {
			const lo = a.lo - b.lo;
			const hi = a.hi - b.hi - (lo < 0 ? 1 : 0);
			if (hi < hmin || hi > hmax) panic("attempt to subtract with overflow");
			return make(hi, lo >>> 0);
		},
		wrappingAdd(a, b) {
			const lo = a.lo + b.lo;
			return make(high(a.hi + b.hi + (lo >= TWO32 ? 1 : 0)), lo >>> 0);
		},
		wrappingSub(a, b) {
			const lo = a.lo - b.lo;
			return make(high(a.hi - b.hi - (lo < 0 ? 1 : 0)), lo >>> 0);
		},
		mul(a, b) {
			if (!signed && (a.hi === 0 || b.hi === 0)) {
				// One factor below 2^32: its product with the other's high half must be too.
				const cross = a.hi * b.lo + a.lo * b.hi;
				const lo = mul32(a.lo, b.lo);
				const hi = mulHi + cross;
				if (hi > 0xffffffff) panic("attempt to multiply with overflow");
				return make(hi, lo);
			}
			return base.mul(a, b);
		},
		div(a, b) {
			if (isZero(b)) panic("attempt to divide by zero");
			if (!signed && b.hi === 0) {
				// Long division by a uint32 in 32- and 16-bit digits, every step exact.
				const d = b.lo;
				const qh = Math.floor(a.hi / d);
				let r = a.hi - qh * d;
				const t1 = r * 65536 + (a.lo >>> 16);
				const q1 = Math.floor(t1 / d);
				r = t1 - q1 * d;
				const q2 = Math.floor((r * 65536 + (a.lo & 0xffff)) / d);
				return make(qh, (q1 * 65536 + q2) >>> 0);
			}
			return base.div(a, b);
		},
		neg(a) {
			if (!signed || (a.hi === hmin && a.lo === 0)) return base.neg(a);
			const lo = -a.lo >>> 0;
			return make((~a.hi + (lo === 0 ? 1 : 0)) | 0, lo);
		},
		not: (a) => make(high(~a.hi), ~a.lo >>> 0),
		and: (a, b) => make(high(a.hi & b.hi), (a.lo & b.lo) >>> 0),
		or: (a, b) => make(high(a.hi | b.hi), (a.lo | b.lo) >>> 0),
		xor: (a, b) => make(high(a.hi ^ b.hi), (a.lo ^ b.lo) >>> 0),
		shl(a, b) {
			const n = shift(b, 64, "left");
			if (n === 0) return a;
			if (n < 32) return make(high((a.hi << n) | (a.lo >>> (32 - n))), (a.lo << n) >>> 0);
			return make(high(a.lo << (n - 32)), 0);
		},
		shr(a, b) {
			const n = shift(b, 64, "right");
			if (n === 0) return a;
			if (signed) {
				if (n < 32) return make(a.hi >> n, ((a.lo >>> n) | (a.hi << (32 - n))) >>> 0);
				return make(a.hi >> 31, (a.hi >> (n - 32)) >>> 0);
			}
			if (n < 32) return make(a.hi >>> n, ((a.lo >>> n) | (a.hi << (32 - n))) >>> 0);
			return make(0, a.hi >>> (n - 32));
		},
		leadingZeros(a) {
			const hi = a.hi >>> 0;
			return hi !== 0 ? Math.clz32(hi) : 32 + Math.clz32(a.lo);
		},
		leadingOnes(a) {
			const hi = ~a.hi >>> 0;
			return hi !== 0 ? Math.clz32(hi) : 32 + Math.clz32(~a.lo >>> 0);
		},
		fromBeBytes: (b) =>
			make(high((b[0] << 24) | (b[1] << 16) | (b[2] << 8) | b[3]), ((b[4] << 24) | (b[5] << 16) | (b[6] << 8) | b[7]) >>> 0),
		toBeBytes(a) {
			const out = new Uint8Array(8);
			const { hi, lo } = a;
			out[0] = hi >>> 24;
			out[1] = hi >>> 16;
			out[2] = hi >>> 8;
			out[3] = hi;
			out[4] = lo >>> 24;
			out[5] = lo >>> 16;
			out[6] = lo >>> 8;
			out[7] = lo;
			return out;
		},
		of: (value) => (typeof value === "number" ? fromNumber(value) : box(value)),
		cast(value) {
			if (typeof value === "number") return fromNumber(value);
			return make(high("w1" in value ? value.w1 : value.hi), value.lo);
		},
		eq: (a, b) => a.hi === b.hi && a.lo === b.lo,
		ne: (a, b) => a.hi !== b.hi || a.lo !== b.lo,
		lt,
		le: (a, b) => !lt(b, a),
		gt: (a, b) => lt(b, a),
		ge: (a, b) => !lt(a, b),
		max: (a, b) => (lt(b, a) ? a : b),
		min: (a, b) => (lt(b, a) ? b : a),
	};
}

/** `u128` or `i128`: four limbs, computed through bigints. */
function wide128<T extends U128Limbs | I128Limbs>(signed: boolean): WideInt<T> {
	const limb = (v: bigint, at: bigint) => Number(BigInt.asUintN(32, v >> at));
	const box = (v: bigint) => {
		const top = v >> 96n;
		const hi = Number(signed ? BigInt.asIntN(32, top) : BigInt.asUintN(32, top));
		return { hi, w2: limb(v, 64n), w1: limb(v, 32n), lo: limb(v, 0n) } as T;
	};
	const compare = (a: T, b: T) => {
		if (a.hi !== b.hi) return a.hi < b.hi ? -1 : 1;
		if (a.w2 !== b.w2) return a.w2 < b.w2 ? -1 : 1;
		if (a.w1 !== b.w1) return a.w1 < b.w1 ? -1 : 1;
		return a.lo === b.lo ? 0 : a.lo < b.lo ? -1 : 1;
	};
	return wide(128, signed, box, compare);
}

/**
 * A wide integer type, its values numbers while they are safe integers: their arithmetic
 * stays in numbers while its result does, and falls back on `limbs` past that.
 */
function dual<T extends Limbs>(limbs: WideInt<T>, signed: boolean): WideInt<number | T> {
	type V = number | T;
	const most = Number.MAX_SAFE_INTEGER;
	const least = signed ? -most : 0;
	const big = (v: V): T => (typeof v === "number" ? limbs.of(v) : v);
	// The one form of a value: a number if it is a safe integer.
	const one = (v: T): V => {
		const n = approx(v);
		return safe(n) ? n : v;
	};
	const fits = (n: number) => n <= most && n >= least;
	const lt = (a: V, b: V) => (typeof a === "number" && typeof b === "number" ? a < b : limbs.lt(big(a), big(b)));
	const eq = (a: V, b: V) => a === b || (typeof a !== "number" && typeof b !== "number" && limbs.eq(a, b));
	// A bitwise operator on two safe integers, a 32-bit half at a time.
	const halves = (a: number, b: number, op: (x: number, y: number) => number) =>
		op(Math.floor(a / TWO32), Math.floor(b / TWO32)) * TWO32 + (op(a >>> 0, b >>> 0) >>> 0);
	const binary = (f: (a: T, b: T) => T) => (a: V, b: V) => one(f(big(a), big(b)));
	const checked = (f: (a: T, b: T) => T | undefined) => (a: V, b: V) => {
		const value = f(big(a), big(b));
		return value === undefined ? undefined : one(value);
	};
	return {
		MIN: one(limbs.MIN),
		MAX: one(limbs.MAX),
		BITS: limbs.BITS,
		add(a, b) {
			if (typeof a === "number" && typeof b === "number" && fits(a + b)) return a + b;
			return one(limbs.add(big(a), big(b)));
		},
		sub(a, b) {
			if (typeof a === "number" && typeof b === "number" && fits(a - b)) return a - b;
			return one(limbs.sub(big(a), big(b)));
		},
		mul(a, b) {
			// `+ 0` turns -0 into 0.
			if (typeof a === "number" && typeof b === "number" && safe(a * b)) return a * b + 0;
			return one(limbs.mul(big(a), big(b)));
		},
		div(a, b) {
			// Below 2^53, a quotient of doubles truncates to the exact one.
			if (typeof a === "number" && typeof b === "number" && b !== 0) return Math.trunc(a / b) + 0;
			return one(limbs.div(big(a), big(b)));
		},
		rem(a, b) {
			if (typeof a === "number" && typeof b === "number" && b !== 0) return (a % b) + 0;
			return one(limbs.rem(big(a), big(b)));
		},
		neg(a) {
			if (signed && typeof a === "number") return -a + 0;
			return one(limbs.neg(big(a)));
		},
		shl(a, b) {
			const n = shift(b, limbs.BITS, "left");
			if (typeof a === "number" && safe(a * 2 ** n)) return a * 2 ** n;
			return one(limbs.shl(big(a), n));
		},
		shr(a, b) {
			const n = shift(b, limbs.BITS, "right");
			if (typeof a === "number") return Math.floor(a / 2 ** n) + 0;
			return one(limbs.shr(big(a), n));
		},
		and: (a, b) => (typeof a === "number" && typeof b === "number" ? halves(a, b, (x, y) => x & y) : one(limbs.and(big(a), big(b)))),
		or: (a, b) => (typeof a === "number" && typeof b === "number" ? halves(a, b, (x, y) => x | y) : one(limbs.or(big(a), big(b)))),
		xor: (a, b) => (typeof a === "number" && typeof b === "number" ? halves(a, b, (x, y) => x ^ y) : one(limbs.xor(big(a), big(b)))),
		not: (a) => (signed && typeof a === "number" ? -a - 1 : one(limbs.not(big(a)))),
		checkedAdd: checked(limbs.checkedAdd),
		checkedSub: checked(limbs.checkedSub),
		checkedMul: checked(limbs.checkedMul),
		saturatingAdd: binary(limbs.saturatingAdd),
		saturatingSub: binary(limbs.saturatingSub),
		wrappingAdd: binary(limbs.wrappingAdd),
		wrappingSub: binary(limbs.wrappingSub),
		leadingOnes: (a) => limbs.leadingOnes(big(a)),
		leadingZeros: (a) => limbs.leadingZeros(big(a)),
		fromBeBytes: (bytes) => one(limbs.fromBeBytes(bytes)),
		toBeBytes: (a) => limbs.toBeBytes(big(a)),
		tryFrom(value) {
			const result = limbs.tryFrom(value);
			return result.ok ? Ok(one(result.value)) : result;
		},
		display: (a, f) => (typeof a === "number" ? f.padIntegral(a >= 0, "", String(a < 0 ? -a : a)) : limbs.display(a, f)),
		lowerHex: (a, f) => limbs.lowerHex(big(a), f),
		upperHex: (a, f) => limbs.upperHex(big(a), f),
		of(value) {
			if (typeof value === "number") {
				if (!Number.isSafeInteger(value)) throw new RangeError(`rust2-ts: ${value} is not a safe integer`);
				if (fits(value)) return value;
			}
			return one(limbs.of(value));
		},
		cast: (value) => (typeof value === "number" && fits(value) ? value : one(limbs.cast(value))),
		low: (a) => (typeof a === "number" ? a >>> 0 : a.lo),
		eq,
		ne: (a, b) => !eq(a, b),
		lt,
		le: (a, b) => !lt(b, a),
		gt: (a, b) => lt(b, a),
		ge: (a, b) => !lt(a, b),
		max: (a, b) => (lt(b, a) ? a : b),
		min: (a, b) => (lt(b, a) ? b : a),
	};
}

export const u8: Int<number> = small(8, false);
export const u16: Int<number> = small(16, false);
export const u32: Int<number> = small(32, false);
export const i8: Int<number> = small(8, true);
export const i16: Int<number> = small(16, true);
export const i32: Int<number> = small(32, true);
/** 32 bits, as on wasm32. */
export const usize: Int<number> = u32;
/** 32 bits, as on wasm32. */
export const isize: Int<number> = i32;
export const u64: WideInt<U64> = dual(wide64<U64Limbs>(false), false);
export const i64: WideInt<I64> = dual(wide64<I64Limbs>(true), true);
export const u128: WideInt<U128> = dual(wide128<U128Limbs>(false), false);
export const i128: WideInt<I128> = dual(wide128<I128Limbs>(true), true);

/** `a < b` of two integers of one type: numbers, or a wide type's limbs. */
export function lt(a: number | Limbs, b: number | Limbs): boolean {
	if (typeof a === "number" && typeof b === "number") return a < b;
	return (typeof a === "number" ? BigInt(a) : exact(a)) < (typeof b === "number" ? BigInt(b) : exact(b));
}

/** The larger of two integers. */
export function max(a: number, b: number): number {
	return a >= b ? a : b;
}

/** The smaller of two integers. */
export function min(a: number, b: number): number {
	return a <= b ? a : b;
}

/** `core::num::TryFromIntError`: which way the value was out of range. */
export class TryFromIntError {
	readonly kind: "PosOverflow" | "NegOverflow";

	constructor(kind: "PosOverflow" | "NegOverflow") {
		this.kind = kind;
	}
}

// Results

/** `Result<T, E>`. */
export type Result<T, E> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: E };

export function Ok<T, E = never>(value: T): Result<T, E> {
	return { ok: true, value };
}

export function Err<E, T = never>(error: E): Result<T, E> {
	return { ok: false, error };
}

function unwrapFailed<T>(message: string, value: T, debug: Debug<T>): never {
	panic([message, ": ", (f) => debug(value, f)]);
}

export const result = {
	isOk<T, E>(r: Result<T, E>): boolean {
		return r.ok;
	},
	isErr<T, E>(r: Result<T, E>): boolean {
		return !r.ok;
	},
	ok<T, E>(r: Result<T, E>): T | undefined {
		return r.ok ? r.value : undefined;
	},
	err<T, E>(r: Result<T, E>): E | undefined {
		return r.ok ? undefined : r.error;
	},
	map<T, E, U>(r: Result<T, E>, f: (value: T) => U): Result<U, E> {
		return r.ok ? Ok(f(r.value)) : r;
	},
	mapErr<T, E, F>(r: Result<T, E>, f: (error: E) => F): Result<T, F> {
		return r.ok ? r : Err(f(r.error));
	},
	andThen<T, E, U>(r: Result<T, E>, f: (value: T) => Result<U, E>): Result<U, E> {
		return r.ok ? f(r.value) : r;
	},
	inspectErr<T, E>(r: Result<T, E>, f: (error: E) => void): Result<T, E> {
		if (!r.ok) f(r.error);
		return r;
	},
	unwrapOr<T, E>(r: Result<T, E>, fallback: T): T {
		return r.ok ? r.value : fallback;
	},
	/** `unwrap`, which shows the error with `debug`. */
	unwrap<T, E>(r: Result<T, E>, debug: Debug<E>): T {
		if (!r.ok) unwrapFailed("called `Result::unwrap()` on an `Err` value", r.error, debug);
		return r.value;
	},
	expect<T, E>(r: Result<T, E>, message: string, debug: Debug<E>): T {
		if (!r.ok) unwrapFailed(message, r.error, debug);
		return r.value;
	},
	unwrapErr<T, E>(r: Result<T, E>, debug: Debug<T>): E {
		if (r.ok) unwrapFailed("called `Result::unwrap_err()` on an `Ok` value", r.value, debug);
		return r.error;
	},
};

// Options, which are `T | undefined`.

export const option = {
	map<T, U>(o: T | undefined, f: (value: T) => U): U | undefined {
		return o === undefined ? undefined : f(o);
	},
	unwrapOr<T>(o: T | undefined, fallback: T): T {
		return o === undefined ? fallback : o;
	},
	mapOr<T, U>(o: T | undefined, fallback: U, f: (value: T) => U): U {
		return o === undefined ? fallback : f(o);
	},
	okOr<T, E>(o: T | undefined, error: E): Result<T, E> {
		return o === undefined ? Err(error) : Ok(o);
	},
	unwrap<T>(o: T | undefined): T {
		if (o === undefined) panic("called `Option::unwrap()` on a `None` value");
		return o;
	},
	expect<T>(o: T | undefined, message: string): T {
		if (o === undefined) panic(message);
		return o;
	},
};

/** `bool::then_some`: `value` if `cond`, which is evaluated either way. */
export function thenSome<T>(cond: boolean, value: T): T | undefined {
	return cond ? value : undefined;
}

// Slices, which are `Uint8Array` views for bytes and arrays otherwise.

/** A bounds-checked index into a slice. */
export function idx(slice: { readonly length: number }, index: number): number {
	if (index >= slice.length) panic(`index out of bounds: the len is ${slice.length} but the index is ${index}`);
	return index;
}

/** A bounds-checked read of a slice. */
export function at<T>(slice: ArrayLike<T>, index: number): T {
	return slice[idx(slice, index)];
}

export const slice = {
	/** `&s[start..end]`, with `end` defaulting to the length. */
	range<S extends Uint8Array | unknown[]>(s: S, start: number, end: number = s.length): S {
		if (start > s.length) panic(`range start index ${start} out of range for slice of length ${s.length}`);
		if (end > s.length) panic(`range end index ${end} out of range for slice of length ${s.length}`);
		if (start > end) panic(`slice index starts at ${start} but ends at ${end}`);
		return (s instanceof Uint8Array ? s.subarray(start, end) : s.slice(start, end)) as S;
	},
	splitFirst<S extends Uint8Array | unknown[]>(s: S): [S[number], S] | undefined {
		return s.length === 0 ? undefined : [s[0], slice.range(s, 1)];
	},
	splitFirstChunk<S extends Uint8Array | unknown[]>(s: S, n: number): [S, S] | undefined {
		return s.length < n ? undefined : [slice.range(s, 0, n), slice.range(s, n)];
	},
	splitAt<S extends Uint8Array | unknown[]>(s: S, n: number): [S, S] {
		if (n > s.length) panic("mid > len");
		return [slice.range(s, 0, n), slice.range(s, n)];
	},
	splitAtChecked<S extends Uint8Array | unknown[]>(s: S, n: number): [S, S] | undefined {
		return n > s.length ? undefined : [slice.range(s, 0, n), slice.range(s, n)];
	},
	first<T>(s: ArrayLike<T>): T | undefined {
		return s.length === 0 ? undefined : s[0];
	},
	get<T>(s: ArrayLike<T>, index: number): T | undefined {
		return index < s.length ? s[index] : undefined;
	},
	copyFromSlice(dst: Uint8Array, src: Uint8Array): void {
		if (dst.length !== src.length) {
			panic(
				`copy_from_slice: source slice length (${src.length}) does not match destination slice length (${dst.length})`,
			);
		}
		dst.set(src);
	},
	copyWithin(s: Uint8Array, start: number, end: number, dest: number): void {
		// `slice::range` checks the end first, then `range` reports the rest.
		if (end > s.length) panic(`range end index ${end} out of range for slice of length ${s.length}`);
		slice.range(s, start, end);
		if (dest > s.length - (end - start)) panic("dest is out of bounds");
		s.copyWithin(dest, start, end);
	},
	/** `to_vec` of bytes. */
	toByteVec(s: Uint8Array): ByteVec {
		return ByteVec.from(s);
	},
	/** `to_vec` of `Copy` elements. */
	toVec<T>(s: readonly T[]): T[] {
		return s.map(copy);
	},
};

/** `Vec<u8>`: a growable byte buffer. */
export class ByteVec {
	/** The storage; only the first `length` bytes are the vector. */
	data: Uint8Array;
	length = 0;

	constructor(capacity = 0) {
		this.data = new Uint8Array(capacity);
	}

	static from(bytes: Uint8Array): ByteVec {
		const v = new ByteVec(bytes.length);
		v.data.set(bytes);
		v.length = bytes.length;
		return v;
	}

	/** Make room for `additional` more bytes. */
	reserve(additional: number): void {
		const needed = this.length + additional;
		if (needed <= this.data.length) return;
		const grown = new Uint8Array(Math.max(needed, this.data.length * 2, 8));
		// All of `data`, not a view of it: a view of a small array makes V8 move it off its
		// heap, which costs more than copying the few bytes past `length`.
		grown.set(this.data);
		this.data = grown;
	}

	/** The bytes, as a view that a later push may detach. */
	view(): Uint8Array {
		return this.data.subarray(0, this.length);
	}

	push(byte: number): void {
		this.reserve(1);
		this.data[this.length++] = byte;
	}

	pop(): number | undefined {
		return this.length === 0 ? undefined : this.data[--this.length];
	}

	extendFromSlice(bytes: Uint8Array): void {
		this.reserve(bytes.length);
		this.data.set(bytes, this.length);
		this.length += bytes.length;
	}

	truncate(length: number): void {
		if (length < this.length) this.length = length;
	}

	clear(): void {
		this.length = 0;
	}
}

/** `vec![value; n]` of bytes. */
export function byteVecOf(value: number, n: number): ByteVec {
	const v = new ByteVec(n);
	v.data.fill(value);
	v.length = n;
	return v;
}

// Strings

const encoder = new TextEncoder();
// `ignoreBOM` keeps a leading U+FEFF, as Rust does.
const decoder = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });

/** `alloc::string::FromUtf8Error`: the bytes that were not UTF-8, and where they stop being. */
export class FromUtf8Error {
	readonly bytes: ByteVec;
	/** `Utf8Error::valid_up_to`. */
	readonly validUpTo: number;
	/** `Utf8Error::error_len`: `undefined` if the input ended mid-character. */
	readonly errorLen: number | undefined;

	constructor(bytes: ByteVec) {
		this.bytes = bytes;
		[this.validUpTo, this.errorLen] = utf8Error(bytes.view());
	}
}

/** Where `v` stops being UTF-8 and how many bytes are invalid there, as std reports it. */
function utf8Error(v: Uint8Array): [number, number | undefined] {
	const tail = (b: number | undefined) => b !== undefined && b >= 0x80 && b <= 0xbf;
	for (let i = 0; i < v.length; ) {
		const first = v[i];
		const width = first < 0x80 ? 1 : first >= 0xc2 && first <= 0xdf ? 2 : first >= 0xe0 && first <= 0xef ? 3 : first >= 0xf0 && first <= 0xf4 ? 4 : 0;
		if (width === 1) {
			i++;
			continue;
		}
		if (width === 0) return [i, 1];
		const second = v[i + 1];
		if (second === undefined) return [i, undefined];
		const [lo, hi] =
			first === 0xe0 ? [0xa0, 0xbf] : first === 0xed ? [0x80, 0x9f] : first === 0xf0 ? [0x90, 0xbf] : first === 0xf4 ? [0x80, 0x8f] : [0x80, 0xbf];
		if (second < lo || second > hi) return [i, 1];
		for (let k = 2; k < width; k++) {
			if (i + k >= v.length) return [i, undefined];
			if (!tail(v[i + k])) return [i, k];
		}
		i += width;
	}
	throw new Error("utf8Error: the bytes are valid UTF-8");
}

export const string = {
	fromUtf8(bytes: ByteVec): Result<string, FromUtf8Error> {
		try {
			return Ok(decoder.decode(bytes.view()));
		} catch {
			return Err(new FromUtf8Error(bytes));
		}
	},
	asBytes(s: string): Uint8Array {
		return encoder.encode(s);
	},
	len(s: string): number {
		return encoder.encode(s).length;
	},
};

// Values

/**
 * A copy of a `Copy` value. Objects stand for structs, tuples, and arrays, and Rust
 * copies those where JS would share them.
 */
export function copy<T>(value: T): T {
	if (typeof value !== "object" || value === null) return value;
	if (value instanceof Uint8Array) return value.slice() as T;
	if (Array.isArray(value)) return value.map(copy) as T;
	const out = Object.create(Object.getPrototypeOf(value));
	for (const key of Object.keys(value)) out[key] = copy((value as Record<string, unknown>)[key]);
	return out;
}

/** `std::mem::replace`: the place's old value, after setting it to `value`. */
export function replace<T>(get: () => T, set: (value: T) => void, value: T): T {
	const old = get();
	set(value);
	return old;
}

// Formatting, as `core::fmt` does it: the same output byte for byte.

/** `core::fmt::Error`. */
export class FmtError {}

/** `fmt::Result`. */
export type FmtResult = Result<undefined, FmtError>;

const written: FmtResult = Ok(undefined);

/** A value ready to write: `&dyn Display`, `&dyn Debug`, or one placeholder's argument. */
export type Fmt = (f: Formatter) => FmtResult;

/** A type's `Debug` or `Display`, as a function of its values. */
export type Debug<T> = (value: T, f: Formatter) => FmtResult;

/** `fmt::Arguments`: literal pieces, and the values written between them. */
export type Arguments = readonly (string | Fmt)[];

/** One placeholder's options: `{:*^9}` is `{ fill: "*", align: "^", width: 9 }`. */
export interface Spec {
	readonly fill?: string;
	readonly align?: "<" | "^" | ">";
	/** The `#` flag: `{:#?}` and `{:#x}`. */
	readonly alternate?: boolean;
	/** The `0` flag: zeros between the sign and the digits. */
	readonly zero?: boolean;
	readonly width?: number;
}

/** `dyn fmt::Write`: where a `Formatter` writes. */
export interface Write {
	writeStr(s: string): FmtResult;
}

class StringWriter implements Write {
	s = "";

	writeStr(s: string): FmtResult {
		this.s += s;
		return written;
	}
}

/** Indents each line it writes, for `{:#?}`'s nesting. */
class PadAdapter implements Write {
	private onNewline = true;
	private readonly out: Write;

	constructor(out: Write) {
		this.out = out;
	}

	writeStr(s: string): FmtResult {
		for (const line of s.split(/(?<=\n)/)) {
			if (line === "") continue;
			if (this.onNewline) {
				const r = this.out.writeStr("    ");
				if (!r.ok) return r;
			}
			this.onNewline = line.endsWith("\n");
			const r = this.out.writeStr(line);
			if (!r.ok) return r;
		}
		return written;
	}
}

/** `fmt::write`: each placeholder gets a fresh `Formatter` with only its own options. */
function write(out: Write, args: Arguments): FmtResult {
	for (const part of args) {
		const r = typeof part === "string" ? out.writeStr(part) : part(new Formatter(out));
		if (!r.ok) return r;
	}
	return written;
}

/** The number of chars in `s`, as Rust counts them for padding. */
function chars(s: string): number {
	let n = 0;
	for (const _ of s) n++;
	return n;
}

/** `core::fmt::Formatter`. */
export class Formatter {
	readonly out: Write;
	readonly spec: Spec;

	constructor(out: Write, spec: Spec = {}) {
		this.out = out;
		this.spec = spec;
	}

	writeStr(s: string): FmtResult {
		return this.out.writeStr(s);
	}

	writeFmt(args: Arguments): FmtResult {
		return write(this.out, args);
	}

	alternate(): boolean {
		return this.spec.alternate === true;
	}

	width(): number | undefined {
		return this.spec.width;
	}

	/** Write `s`, padded to the width. */
	pad(s: string): FmtResult {
		const width = this.spec.width;
		const count = width === undefined ? 0 : chars(s);
		if (width === undefined || count >= width) return this.out.writeStr(s);
		return this.padded(width - count, this.spec.align ?? "<", this.spec.fill ?? " ", s);
	}

	/** Write an integer's sign, `prefix` if alternate, and `digits`, padded to the width. */
	padIntegral(nonnegative: boolean, prefix: string, digits: string): FmtResult {
		const head = (nonnegative ? "" : "-") + (this.alternate() ? prefix : "");
		const length = head.length + digits.length;
		const width = this.spec.width ?? 0;
		if (length >= width) return this.out.writeStr(head + digits);
		if (this.spec.zero) {
			const r = this.out.writeStr(head);
			return r.ok ? this.padded(width - length, ">", "0", digits) : r;
		}
		return this.padded(width - length, this.spec.align ?? ">", this.spec.fill ?? " ", head + digits);
	}

	/** Write `s` with `padding` fills around it. */
	private padded(padding: number, align: "<" | "^" | ">", fill: string, s: string): FmtResult {
		const left = align === "<" ? 0 : align === ">" ? padding : Math.floor(padding / 2);
		const r = this.out.writeStr(fill.repeat(left) + s);
		return r.ok ? this.out.writeStr(fill.repeat(padding - left)) : r;
	}

	/** This formatter's options, writing to `out`. */
	wrap(out: Write): Formatter {
		return new Formatter(out, this.spec);
	}

	debugStruct(name: string): DebugStruct {
		return new DebugStruct(this, name);
	}

	debugTuple(name: string): DebugTuple {
		return new DebugTuple(this, name);
	}

	debugList(): DebugList {
		return new DebugList(this);
	}

	/** `debug_struct_fields_finish`, which derived `Debug` calls past five fields. */
	debugStructFields(name: string, names: readonly string[], values: readonly Fmt[]): FmtResult {
		if (names.length !== values.length) {
			assertFailed(
				"==",
				(f) => usize.display(names.length, f),
				(f) => usize.display(values.length, f),
			);
		}
		const builder = this.debugStruct(name);
		names.forEach((field, i) => builder.field(field, values[i]));
		return builder.finish();
	}

	/** `debug_tuple_fields_finish`, which derived `Debug` calls past five fields. */
	debugTupleFields(name: string, values: readonly Fmt[]): FmtResult {
		const builder = this.debugTuple(name);
		for (const value of values) builder.field(value);
		return builder.finish();
	}
}

/** `fmt::DebugStruct`. */
export class DebugStruct {
	private readonly fmt: Formatter;
	private result: FmtResult;
	private hasFields = false;

	constructor(fmt: Formatter, name: string) {
		this.fmt = fmt;
		this.result = fmt.writeStr(name);
	}

	field(name: string, value: Fmt): this {
		if (this.result.ok) {
			if (this.fmt.alternate()) {
				const head = this.hasFields ? written : this.fmt.writeStr(" {\n");
				this.result = head.ok ? pretty(this.fmt, [name, ": ", value, ",\n"]) : head;
			} else {
				this.result = this.fmt.writeStr((this.hasFields ? ", " : " { ") + name + ": ");
				if (this.result.ok) this.result = value(this.fmt);
			}
		}
		this.hasFields = true;
		return this;
	}

	finishNonExhaustive(): FmtResult {
		if (this.result.ok) {
			if (!this.hasFields) this.result = this.fmt.writeStr(" { .. }");
			else if (!this.fmt.alternate()) this.result = this.fmt.writeStr(", .. }");
			else this.result = closePretty(this.fmt, "}");
		}
		return this.result;
	}

	finish(): FmtResult {
		if (this.hasFields && this.result.ok) this.result = this.fmt.writeStr(this.fmt.alternate() ? "}" : " }");
		return this.result;
	}
}

/** `fmt::DebugTuple`. */
export class DebugTuple {
	private readonly fmt: Formatter;
	private result: FmtResult;
	private fields = 0;
	private readonly emptyName: boolean;

	constructor(fmt: Formatter, name: string) {
		this.fmt = fmt;
		this.result = fmt.writeStr(name);
		this.emptyName = name === "";
	}

	field(value: Fmt): this {
		if (this.result.ok) {
			if (this.fmt.alternate()) {
				const head = this.fields > 0 ? written : this.fmt.writeStr("(\n");
				this.result = head.ok ? pretty(this.fmt, [value, ",\n"]) : head;
			} else {
				this.result = this.fmt.writeStr(this.fields === 0 ? "(" : ", ");
				if (this.result.ok) this.result = value(this.fmt);
			}
		}
		this.fields++;
		return this;
	}

	finishNonExhaustive(): FmtResult {
		if (this.result.ok) {
			if (this.fields === 0) this.result = this.fmt.writeStr("(..)");
			else if (!this.fmt.alternate()) this.result = this.fmt.writeStr(", ..)");
			else this.result = closePretty(this.fmt, ")");
		}
		return this.result;
	}

	finish(): FmtResult {
		if (this.fields > 0 && this.result.ok) {
			const comma = this.fields === 1 && this.emptyName && !this.fmt.alternate();
			this.result = this.fmt.writeStr(comma ? ",)" : ")");
		}
		return this.result;
	}
}

/** `fmt::DebugList`. */
export class DebugList {
	private readonly fmt: Formatter;
	private result: FmtResult;
	private hasFields = false;

	constructor(fmt: Formatter) {
		this.fmt = fmt;
		this.result = fmt.writeStr("[");
	}

	entry(value: Fmt): this {
		if (this.result.ok) {
			if (this.fmt.alternate()) {
				const head = this.hasFields ? written : this.fmt.writeStr("\n");
				this.result = head.ok ? pretty(this.fmt, [value, ",\n"]) : head;
			} else {
				this.result = this.hasFields ? this.fmt.writeStr(", ") : written;
				if (this.result.ok) this.result = value(this.fmt);
			}
		}
		this.hasFields = true;
		return this;
	}

	entries<T>(items: Iterable<T>, debug: Debug<T>): this {
		for (const item of items) this.entry((f) => debug(item, f));
		return this;
	}

	finishNonExhaustive(): FmtResult {
		if (!this.result.ok) return this.result;
		if (!this.hasFields) return this.fmt.writeStr("..]");
		if (!this.fmt.alternate()) return this.fmt.writeStr(", ..]");
		return closePretty(this.fmt, "]");
	}

	finish(): FmtResult {
		if (this.result.ok) this.result = this.fmt.writeStr("]");
		return this.result;
	}
}

/** Write `parts` indented, as a pretty builder writes one field: values keep `f`'s options. */
function pretty(f: Formatter, parts: Arguments): FmtResult {
	const writer = f.wrap(new PadAdapter(f.out));
	for (const part of parts) {
		const r = typeof part === "string" ? writer.writeStr(part) : part(writer);
		if (!r.ok) return r;
	}
	return written;
}

/** A pretty builder's `finish_non_exhaustive`: an indented `..`, then `close`. */
function closePretty(f: Formatter, close: string): FmtResult {
	const r = pretty(f, ["..\n"]);
	return r.ok ? f.writeStr(close) : r;
}

// `Debug` and `Display` of what std defines them for. A string's `Display` is `f.pad`.

/** Characters `Debug` of a string writes as `\u{..}`, by std's rule. */
const unprintable = /[\p{Cc}\p{Co}\p{White_Space}\p{Grapheme_Extend}\p{Default_Ignorable_Code_Point}\p{Cf}\P{Assigned}]/u;

/** One char of a string's `Debug`. */
function escapeDebug(c: string): string {
	switch (c) {
		case '"':
			return '\\"';
		case "\\":
			return "\\\\";
		case "\n":
			return "\\n";
		case "\t":
			return "\\t";
		case "\r":
			return "\\r";
		case "\0":
			return "\\0";
	}
	const point = c.codePointAt(0)!;
	// Printable ASCII, and two halfwidth katakana marks that extend graphemes but print.
	if ((point >= 0x20 && point <= 0x7e) || point === 0xff9e || point === 0xff9f) return c;
	return unprintable.test(c) ? `\\u{${point.toString(16)}}` : c;
}

/** `core::fmt`'s functions, and `Debug` and `Display` of std's types. */
export const fmt = {
	/** `format!`, from `alloc::fmt::format`. */
	format(args: Arguments): string {
		const out = new StringWriter();
		if (!write(out, args).ok) {
			panic("a formatting trait implementation returned an error when the underlying stream did not: Error");
		}
		return out.s;
	},
	/** `ToString::to_string` of a `Display` value. */
	toString(value: Fmt): string {
		const out = new StringWriter();
		if (!value(new Formatter(out)).ok) panic("a Display implementation returned an error unexpectedly: Error");
		return out.s;
	},
	/** A placeholder with options, which replace the defaults its value is written with. */
	spec(spec: Spec, value: Fmt): Fmt {
		return (f) => value(new Formatter(f.out, spec));
	},
	bool(value: boolean, f: Formatter): FmtResult {
		return f.pad(value ? "true" : "false");
	},
	/** `Debug` of a string. */
	str(s: string, f: Formatter): FmtResult {
		let out = '"';
		for (const c of s) out += escapeDebug(c);
		return f.writeStr(out + '"');
	},
	/** `Debug` of `()`. */
	unit(_: undefined, f: Formatter): FmtResult {
		return f.pad("()");
	},
	option<T>(o: T | undefined, f: Formatter, debug: Debug<T>): FmtResult {
		if (o === undefined) return f.writeStr("None");
		return f
			.debugTuple("Some")
			.field((f) => debug(o, f))
			.finish();
	},
	result<T, E>(r: Result<T, E>, f: Formatter, ok: Debug<T>, err: Debug<E>): FmtResult {
		const field: Fmt = r.ok ? (f) => ok(r.value, f) : (f) => err(r.error, f);
		return f
			.debugTuple(r.ok ? "Ok" : "Err")
			.field(field)
			.finish();
	},
	/** `Debug` of a slice, array, or `Vec`. */
	list<T>(items: Iterable<T>, f: Formatter, debug: Debug<T>): FmtResult {
		return f.debugList().entries(items, debug).finish();
	},
	/** `Debug` of a tuple, given each element's. */
	tuple(fields: readonly Fmt[], f: Formatter): FmtResult {
		const builder = f.debugTuple("");
		for (const field of fields) builder.field(field);
		return builder.finish();
	},
	/** `Display` of `fmt::Arguments`, which ignores the formatter's options. */
	arguments(args: Arguments, f: Formatter): FmtResult {
		return f.writeFmt(args);
	},
	errorDebug(_: FmtError, f: Formatter): FmtResult {
		return f.writeStr("Error");
	},
	errorDisplay(_: FmtError, f: Formatter): FmtResult {
		return f.pad("an error occurred when formatting an argument");
	},
	tryFromIntErrorDebug(e: TryFromIntError, f: Formatter): FmtResult {
		return f
			.debugTuple("TryFromIntError")
			.field((f) => f.writeStr(e.kind))
			.finish();
	},
	tryFromIntErrorDisplay(e: TryFromIntError, f: Formatter): FmtResult {
		const positive = e.kind === "PosOverflow";
		return f.pad(`number too ${positive ? "large" : "small"} to fit in target type`);
	},
	fromUtf8ErrorDebug(e: FromUtf8Error, f: Formatter): FmtResult {
		const error: Fmt = (f) =>
			f
				.debugStruct("Utf8Error")
				.field("valid_up_to", (f) => usize.display(e.validUpTo, f))
				.field("error_len", (f) => fmt.option(e.errorLen, f, u8.display))
				.finish();
		return f
			.debugStruct("FromUtf8Error")
			.field("bytes", (f) => fmt.list(e.bytes.view(), f, u8.display))
			.field("error", error)
			.finish();
	},
	/** `Display` of the error, which `write!`s and so ignores the formatter's options. */
	fromUtf8ErrorDisplay(e: FromUtf8Error, f: Formatter): FmtResult {
		if (e.errorLen === undefined) return f.writeStr(`incomplete utf-8 byte sequence from index ${e.validUpTo}`);
		return f.writeStr(`invalid utf-8 sequence of ${e.errorLen} bytes from index ${e.validUpTo}`);
	},
};
