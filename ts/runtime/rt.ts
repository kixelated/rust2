// The runtime of rust2-ts output: what Rust's std does that plain JS does not.
//
// Integers up to 32 bits are numbers; wider ones are bigints. Arithmetic is checked as
// a debug build checks it: overflow panics, it never wraps silently.

/** A Rust panic. */
export class Panic extends Error {
	override name = "Panic";
}

/** Panic with `message`. */
export function panic(message: string): never {
	throw new Panic(message);
}

// Integers

/** The checks and methods of one integer type, called to check a value fits it. */
export interface Int<T extends number | bigint> {
	(value: T): T;
	readonly MIN: T;
	readonly MAX: T;
	readonly BITS: number;
	div(a: T, b: T): T;
	rem(a: T, b: T): T;
	shl(a: T, b: number | bigint): T;
	shr(a: T, b: number | bigint): T;
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
	tryFrom(value: number | bigint): Result<T, TryFromIntError>;
}

function shift(b: number | bigint, bits: number): number {
	const n = Number(b);
	if (n < 0 || n >= bits) panic("attempt to shift with overflow");
	return n;
}

function int<T extends number | bigint>(
	bits: number,
	signed: boolean,
	of: (value: number | bigint) => T,
	wrap: (value: bigint) => T,
): Int<T> {
	const min = signed ? -(1n << BigInt(bits - 1)) : 0n;
	const max = signed ? (1n << BigInt(bits - 1)) - 1n : (1n << BigInt(bits)) - 1n;
	const fits = (value: number | bigint) => value >= min && value <= max;
	const check = (value: T): T => {
		if (!fits(value)) panic("attempt to compute with overflow");
		return value;
	};
	const big = (value: T) => BigInt(value);
	const unsigned = (value: T) => BigInt.asUintN(bits, big(value));
	const methods = {
		MIN: of(min),
		MAX: of(max),
		BITS: bits,
		div(a: T, b: T): T {
			if (big(b) === 0n) panic("attempt to divide by zero");
			return check(of(big(a) / big(b)));
		},
		rem(a: T, b: T): T {
			if (big(b) === 0n) panic("attempt to calculate the remainder with a divisor of zero");
			if (signed && big(b) === -1n && big(a) === min) panic("attempt to calculate the remainder with overflow");
			return of(big(a) % big(b));
		},
		shl(a: T, b: number | bigint): T {
			return wrap(big(a) << BigInt(shift(b, bits)));
		},
		shr(a: T, b: number | bigint): T {
			return of(big(a) >> BigInt(shift(b, bits)));
		},
		checkedAdd(a: T, b: T): T | undefined {
			const sum = big(a) + big(b);
			return fits(sum) ? of(sum) : undefined;
		},
		checkedSub(a: T, b: T): T | undefined {
			const diff = big(a) - big(b);
			return fits(diff) ? of(diff) : undefined;
		},
		checkedMul(a: T, b: T): T | undefined {
			const product = big(a) * big(b);
			return fits(product) ? of(product) : undefined;
		},
		saturatingAdd(a: T, b: T): T {
			const sum = big(a) + big(b);
			return of(sum < min ? min : sum > max ? max : sum);
		},
		saturatingSub(a: T, b: T): T {
			const diff = big(a) - big(b);
			return of(diff < min ? min : diff > max ? max : diff);
		},
		wrappingAdd(a: T, b: T): T {
			return wrap(big(a) + big(b));
		},
		wrappingSub(a: T, b: T): T {
			return wrap(big(a) - big(b));
		},
		leadingOnes(a: T): number {
			let n = 0;
			for (let bit = BigInt(bits - 1); bit >= 0n && (unsigned(a) >> bit) & 1n; bit--) n++;
			return n;
		},
		leadingZeros(a: T): number {
			let n = 0;
			for (let bit = BigInt(bits - 1); bit >= 0n && !((unsigned(a) >> bit) & 1n); bit--) n++;
			return n;
		},
		fromBeBytes(bytes: Uint8Array): T {
			let value = 0n;
			for (const byte of bytes) value = (value << 8n) | BigInt(byte);
			return wrap(value);
		},
		toBeBytes(a: T): Uint8Array {
			const out = new Uint8Array(bits / 8);
			let value = unsigned(a);
			for (let i = out.length - 1; i >= 0; i--) {
				out[i] = Number(value & 0xffn);
				value >>= 8n;
			}
			return out;
		},
		tryFrom(value: number | bigint): Result<T, TryFromIntError> {
			return fits(value) ? Ok(of(value)) : Err(new TryFromIntError());
		},
	};
	return Object.assign(check, methods);
}

function small(bits: number, signed: boolean): Int<number> {
	return int(bits, signed, Number, (value) => Number(signed ? BigInt.asIntN(bits, value) : BigInt.asUintN(bits, value)));
}

function wide(bits: number, signed: boolean): Int<bigint> {
	return int(bits, signed, BigInt, (value) => (signed ? BigInt.asIntN(bits, value) : BigInt.asUintN(bits, value)));
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
export const u64: Int<bigint> = wide(64, false);
export const i64: Int<bigint> = wide(64, true);
export const u128: Int<bigint> = wide(128, false);
export const i128: Int<bigint> = wide(128, true);

/** The larger of two integers. */
export function max<T extends number | bigint>(a: T, b: T): T {
	return a >= b ? a : b;
}

/** The smaller of two integers. */
export function min<T extends number | bigint>(a: T, b: T): T {
	return a <= b ? a : b;
}

/** `core::num::TryFromIntError`. */
export class TryFromIntError {}

// Results

/** `Result<T, E>`. */
export type Result<T, E> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: E };

export function Ok<T, E = never>(value: T): Result<T, E> {
	return { ok: true, value };
}

export function Err<E, T = never>(error: E): Result<T, E> {
	return { ok: false, error };
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
	unwrap<T, E>(r: Result<T, E>): T {
		if (!r.ok) panic("called `Result::unwrap()` on an `Err` value");
		return r.value;
	},
	expect<T, E>(r: Result<T, E>, message: string): T {
		if (!r.ok) panic(message);
		return r.value;
	},
	unwrapErr<T, E>(r: Result<T, E>): E {
		if (r.ok) panic("called `Result::unwrap_err()` on an `Ok` value");
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
		if (start > end) panic(`slice index starts at ${start} but ends at ${end}`);
		if (end > s.length) panic(`range end index ${end} out of range for slice of length ${s.length}`);
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
		if (dst.length !== src.length) panic("source slice length does not match destination slice length");
		dst.set(src);
	},
	copyWithin(s: Uint8Array, start: number, end: number, dest: number): void {
		if (start > end || end > s.length) panic("range out of bounds");
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
		grown.set(this.view());
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

/** `alloc::string::FromUtf8Error`: the bytes that were not UTF-8. */
export class FromUtf8Error {
	readonly bytes: ByteVec;

	constructor(bytes: ByteVec) {
		this.bytes = bytes;
	}
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
