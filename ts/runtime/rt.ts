// The runtime of rust2-ts output: what Rust's std does that plain JS does not.
//
// Every operation is its own exported function, named by what it acts on, `sliceRange`
// or `fmtPad`, so a bundler drops every one translated code never calls. Methods are
// left only where a call dispatches on its receiver: an iterator's `next`, a writer's
// `writeStr`, a future's `poll`, a waker's, and `[Symbol.dispose]`.

import { type char, type TryFromIntError, type u32, u8Display, usizeDisplay } from "./rt/int.ts";

export * from "./rt/collections.ts";
export * from "./rt/int.ts";
export * from "./rt/iter.ts";
export * from "./rt/str.ts";
export * from "./rt/task.ts";
export * from "./rt/time.ts";

/** A Rust panic. */
export class Panic extends Error {
	override name = "Panic";
}

/** Panic with `message`. */
export function panic(message: string): never {
	throw new Panic(message);
}

/**
 * Panic with a formatted message, which keeps what was written before a value failed to
 * format, as std's does.
 */
export function panicFmt(message: Arguments): never {
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
	const failed: Arguments = args === undefined ? [" failed"] : [" failed: ", (f) => fmtWriteFmt(f, args)];
	panicFmt(["assertion `left ", op, " right`", ...failed, "\n  left: ", left, "\n right: ", right]);
}

// Places

/**
 * A `&mut` to a value JS copies, such as a number, which no JS value can point at: where
 * the value is, read and written through `v`.
 */
export interface Place<T> {
	v: T;
}

/** A place read and written by `get` and `set`: a local's, a field's, or an element's. */
class Accessed<T> implements Place<T> {
	private readonly read: () => T;
	private readonly write: (value: T) => void;

	constructor(read: () => T, write: (value: T) => void) {
		this.read = read;
		this.write = write;
	}

	get v(): T {
		return this.read();
	}

	set v(value: T) {
		this.write(value);
	}
}

/** The place `get` reads and `set` writes. */
export function place<T>(get: () => T, set: (value: T) => void): Place<T> {
	return new Accessed(get, set);
}

// Results

/** `Result<T, E>`. */
export type Result<T, E> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: E };

/** `Ok(value)`. */
export function Ok<T, E = never>(value: T): Result<T, E> {
	return { ok: true, value };
}

/** `Err(error)`. */
export function Err<E, T = never>(error: E): Result<T, E> {
	return { ok: false, error };
}

/** Panic with `message` and `value`'s `Debug`, and drop `value` as the panic unwinds. */
function unwrapFailed<T>(message: string, value: T, debug: Debug<T>, drop?: (value: T) => void): never {
	try {
		panicFmt([message, ": ", (f) => debug(value, f)]);
	} catch (e) {
		drop?.(value);
		throw e;
	}
}

// A function that may drop an operand std's would, rather than return it, takes what
// drops it, and drops it where std's does: as it returns, after any closure it calls, or
// as a panic in that closure unwinds.

/** `Result::ok`: the value, or `undefined` for an error, which drops. */
export function resultOk<T, E>(r: Result<T, E>, drop?: (error: E) => void): T | undefined {
	if (r.ok) return r.value;
	drop?.(r.error);
	return undefined;
}

/** `Result::err`: the error, or `undefined` for a value, which drops. */
export function resultErr<T, E>(r: Result<T, E>, drop?: (value: T) => void): E | undefined {
	if (!r.ok) return r.error;
	drop?.(r.value);
	return undefined;
}

/** `Result::map`. */
export function resultMap<T, E, U>(r: Result<T, E>, f: (value: T) => U): Result<U, E> {
	return r.ok ? Ok(f(r.value)) : r;
}

/** `Result::map_err`. */
export function resultMapErr<T, E, F>(r: Result<T, E>, f: (error: E) => F): Result<T, F> {
	return r.ok ? r : Err(f(r.error));
}

/** `Result::and_then`. */
export function resultAndThen<T, E, U>(r: Result<T, E>, f: (value: T) => Result<U, E>): Result<U, E> {
	return r.ok ? f(r.value) : r;
}

/** `Result::inspect_err`, whose error drops if `f` panics. */
export function resultInspectErr<T, E>(r: Result<T, E>, f: (error: E) => void, drop?: (error: E) => void): Result<T, E> {
	if (r.ok) return r;
	try {
		f(r.error);
	} catch (e) {
		drop?.(r.error);
		throw e;
	}
	return r;
}

/** `Result::unwrap_or`: the value, and the fallback drops, or else the fallback, and the error drops. */
export function resultUnwrapOr<T, E>(r: Result<T, E>, fallback: T, value?: (value: T) => void, error?: (error: E) => void): T {
	if (r.ok) {
		value?.(fallback);
		return r.value;
	}
	error?.(r.error);
	return fallback;
}

/** `unwrap`, which shows the error with `debug`, and drops it as the panic unwinds. */
export function resultUnwrap<T, E>(r: Result<T, E>, debug: Debug<E>, drop?: (error: E) => void): T {
	if (!r.ok) unwrapFailed("called `Result::unwrap()` on an `Err` value", r.error, debug, drop);
	return r.value;
}

/** `Result::expect`: the value, or a panic with `message` and the error's `Debug`. */
export function resultExpect<T, E>(r: Result<T, E>, message: string, debug: Debug<E>, drop?: (error: E) => void): T {
	if (!r.ok) unwrapFailed(message, r.error, debug, drop);
	return r.value;
}

/** `Result::unwrap_err`: the error, or a panic with the value's `Debug`. */
export function resultUnwrapErr<T, E>(r: Result<T, E>, debug: Debug<T>, drop?: (value: T) => void): E {
	if (r.ok) unwrapFailed("called `Result::unwrap_err()` on an `Ok` value", r.value, debug, drop);
	return r.error;
}

// Options, which are `T | undefined`.

/** `Option::map`. */
export function optionMap<T, U>(o: T | undefined, f: (value: T) => U): U | undefined {
	return o === undefined ? undefined : f(o);
}

/** `Option::unwrap_or`: the value, and the fallback drops, or else the fallback. */
export function optionUnwrapOr<T>(o: T | undefined, fallback: T, drop?: (value: T) => void): T {
	if (o === undefined) return fallback;
	drop?.(fallback);
	return o;
}

// `f` decides `U`, since a literal fallback would infer a literal type.
/** `Option::map_or`: `f` of the value, and the fallback drops, or else the fallback, and `f` drops. */
export function optionMapOr<T, U>(o: T | undefined, fallback: NoInfer<U>, f: (value: T) => U, drop?: (value: U) => void, dropF?: (f: (value: T) => U) => void): U {
	if (o === undefined) {
		dropF?.(f);
		return fallback;
	}
	let mapped: U;
	try {
		mapped = f(o);
	} finally {
		drop?.(fallback);
	}
	return mapped;
}

/** `Option::ok_or`: the value, and the error drops, or else the error. */
export function optionOkOr<T, E>(o: T | undefined, error: E, drop?: (error: E) => void): Result<T, E> {
	if (o === undefined) return Err(error);
	drop?.(error);
	return Ok(o);
}

/** `Option::unwrap`: the value, or a panic. */
export function optionUnwrap<T>(o: T | undefined): T {
	if (o === undefined) panic("called `Option::unwrap()` on a `None` value");
	return o;
}

/** `Option::expect`: the value, or a panic with `message`. */
export function optionExpect<T>(o: T | undefined, message: string): T {
	if (o === undefined) panic(message);
	return o;
}

/** `bool::then_some`: `value` if `cond`, which is evaluated either way, and else drops. */
export function thenSome<T>(cond: boolean, value: T, drop?: (value: T) => void): T | undefined {
	if (cond) return value;
	drop?.(value);
	return undefined;
}

// Not `then`, which would make the module's namespace a thenable that `await` calls.
/** `bool::then`: `f()` if `cond`, which is called only then, and else `f` drops. */
export function boolThen<T>(cond: boolean, f: () => T, drop?: (f: () => T) => void): T | undefined {
	if (cond) return f();
	drop?.(f);
	return undefined;
}

// Slices: arrays, but for bytes, which are `ByteSlice`s.

/** A bounds-checked index into a slice. */
export function idx(slice: { readonly length: number }, index: number): number {
	if (index >= slice.length) panic(`index out of bounds: the len is ${slice.length} but the index is ${index}`);
	return index;
}

/** A bounds-checked read of a slice. */
export function at<T>(slice: ArrayLike<T>, index: number): T {
	return slice[idx(slice, index)];
}

/** Panic unless `start..end` is in a slice of `length`, as std's range checks do. */
function rangeCheck(length: number, start: number, end: number): void {
	if (start > length) panic(`range start index ${start} out of range for slice of length ${length}`);
	if (end > length) panic(`range end index ${end} out of range for slice of length ${length}`);
	if (start > end) panic(`slice index starts at ${start} but ends at ${end}`);
}

/** `&s[start..end]`, with `end` defaulting to the length. */
export function sliceRange<T>(s: T[], start: number, end: number = s.length): T[] {
	rangeCheck(s.length, start, end);
	return s.slice(start, end);
}

/** `split_first`: the first element and the rest, or `undefined` if empty. */
export function sliceSplitFirst<T>(s: T[]): [T, T[]] | undefined {
	return s.length === 0 ? undefined : [s[0], s.slice(1)];
}

/** `split_first_chunk`: the first `n` elements and the rest, or `undefined` if shorter. */
export function sliceSplitFirstChunk<T>(s: T[], n: number): [T[], T[]] | undefined {
	return s.length < n ? undefined : [s.slice(0, n), s.slice(n)];
}

/** `split_at`, panicking past the length. */
export function sliceSplitAt<T>(s: T[], n: number): [T[], T[]] {
	if (n > s.length) panic("mid > len");
	return [s.slice(0, n), s.slice(n)];
}

/** `split_at_checked`: the halves, or `undefined` past the length. */
export function sliceSplitAtChecked<T>(s: T[], n: number): [T[], T[]] | undefined {
	return n > s.length ? undefined : [s.slice(0, n), s.slice(n)];
}

/** `first`: the first element, or `undefined` if empty. */
export function sliceFirst<T>(s: ArrayLike<T>): T | undefined {
	return s.length === 0 ? undefined : s[0];
}

/** `get`: the element at `index`, or `undefined` past the end. */
export function sliceGet<T>(s: ArrayLike<T>, index: number): T | undefined {
	return index < s.length ? s[index] : undefined;
}

/**
 * `&[u8]` and `&mut [u8]`, and `&[u8; N]`: `length` bytes of `data` from `start`. It never changes, so a
 * split makes another, which costs an object where a `Uint8Array` view costs several
 * times as much.
 */
export class ByteSlice {
	readonly data: Uint8Array;
	readonly start: number;
	readonly length: number;

	constructor(data: Uint8Array, start: number, length: number) {
		this.data = data;
		this.start = start;
		this.length = length;
	}
}

/** All of `bytes` as a slice, as `&[u8; N]` coerces to `&[u8]`. */
export function byteSliceFrom(bytes: Uint8Array): ByteSlice {
	return new ByteSlice(bytes, 0, bytes.length);
}

/** A view of the bytes, for what reads a `Uint8Array`. */
export function byteSliceView(s: ByteSlice): Uint8Array {
	return s.data.subarray(s.start, s.start + s.length);
}

/** `&s[start..end]`, with `end` defaulting to the length. */
export function byteSliceRange(s: ByteSlice, start: number, end: number = s.length): ByteSlice {
	rangeCheck(s.length, start, end);
	return new ByteSlice(s.data, s.start + start, end - start);
}

/** `split_first`: the first byte and the rest, or `undefined` if empty. */
export function byteSliceSplitFirst(s: ByteSlice): [number, ByteSlice] | undefined {
	return s.length === 0 ? undefined : [s.data[s.start], new ByteSlice(s.data, s.start + 1, s.length - 1)];
}

/** `split_first_chunk`: the first `n` bytes and the rest, or `undefined` if shorter. */
export function byteSliceSplitFirstChunk(s: ByteSlice, n: number): [ByteSlice, ByteSlice] | undefined {
	if (s.length < n) return undefined;
	return [new ByteSlice(s.data, s.start, n), new ByteSlice(s.data, s.start + n, s.length - n)];
}

/** `*r` of a `&[u8; N]`: a copy of its bytes. */
export function byteSliceToArray(s: ByteSlice): Uint8Array {
	return s.data.slice(s.start, s.start + s.length);
}

/** `split_at`, panicking past the length. */
export function byteSliceSplitAt(s: ByteSlice, n: number): [ByteSlice, ByteSlice] {
	if (n > s.length) panic("mid > len");
	return [new ByteSlice(s.data, s.start, n), new ByteSlice(s.data, s.start + n, s.length - n)];
}

/** `split_at_checked`: the halves, or `undefined` past the length. */
export function byteSliceSplitAtChecked(s: ByteSlice, n: number): [ByteSlice, ByteSlice] | undefined {
	if (n > s.length) return undefined;
	return [new ByteSlice(s.data, s.start, n), new ByteSlice(s.data, s.start + n, s.length - n)];
}

/** `first`: the first byte, or `undefined` if empty. */
export function byteSliceFirst(s: ByteSlice): number | undefined {
	return s.length === 0 ? undefined : s.data[s.start];
}

/** `get`: the byte at `index`, or `undefined` past the end. */
export function byteSliceGet(s: ByteSlice, index: number): number | undefined {
	return index < s.length ? s.data[s.start + index] : undefined;
}

/** `==` of byte slices: the same length and bytes. */
export function byteSliceEq(a: ByteSlice, b: ByteSlice): boolean {
	if (a.length !== b.length) return false;
	for (let i = 0; i < a.length; i++) if (a.data[a.start + i] !== b.data[b.start + i]) return false;
	return true;
}

/** Copy `src` into `dst` at `at`; a few bytes copy faster than a view of them is made. */
function copyBytes(dst: Uint8Array, at: number, src: Uint8Array, start: number, length: number): void {
	if (length <= 16) {
		for (let i = 0; i < length; i++) dst[at + i] = src[start + i];
	} else {
		dst.set(src.subarray(start, start + length), at);
	}
}

/** `copy_from_slice`, panicking if the lengths differ. */
export function byteSliceCopyFromSlice(dst: ByteSlice, src: ByteSlice): void {
	if (dst.length !== src.length) {
		panic(`copy_from_slice: source slice length (${src.length}) does not match destination slice length (${dst.length})`);
	}
	copyBytes(dst.data, dst.start, src.data, src.start, src.length);
}

/** `copy_within`: `s[start..end]` copied to index `dest`, panicking out of bounds. */
export function byteSliceCopyWithin(s: ByteSlice, start: number, end: number, dest: number): void {
	// `slice::range` checks the end first, then `range` reports the rest.
	if (end > s.length) panic(`range end index ${end} out of range for slice of length ${s.length}`);
	rangeCheck(s.length, start, end);
	if (dest > s.length - (end - start)) panic("dest is out of bounds");
	s.data.copyWithin(s.start + dest, s.start + start, s.start + end);
}

/** `to_vec` of bytes. */
export function byteSliceToByteVec(s: ByteSlice): ByteVec {
	const v = new ByteVec(s.length);
	copyBytes(v.data, 0, s.data, s.start, s.length);
	v.length = s.length;
	return v;
}

/** `vec![value; n]`: `n - 1` clones of `value`, in order, then `value` itself; `clone` is `undefined` where a clone is the value. */
export function vecFromElem<T>(value: T, n: number, clone: ((value: T) => T) | undefined): T[] {
	const v: T[] = [];
	for (let i = 1; i < n; i++) v.push(clone === undefined ? value : clone(value));
	if (n > 0) v.push(value);
	return v;
}

const SLAB = 8192;
let slab = /* @__PURE__ */ new ArrayBuffer(SLAB);
let slabUsed = 0;

/**
 * `n` zeroed bytes. Up to 1 KiB, they share an `ArrayBuffer`, as Node's `Buffer` pool
 * does: a lone array's own buffer costs V8 about a microsecond, made when the array is
 * past 64 bytes or else the first time something views it. No byte is handed out twice.
 */
export function bytes(n: number): Uint8Array {
	if (n > SLAB / 8) return new Uint8Array(n);
	if (slabUsed + n > SLAB) {
		slab = new ArrayBuffer(SLAB);
		slabUsed = 0;
	}
	const out = new Uint8Array(slab, slabUsed, n);
	// Aligned, as Node aligns its pool.
	slabUsed += (n + 7) & ~7;
	return out;
}

/** `Vec<u8>`: a growable byte buffer. */
export class ByteVec {
	/** The storage; only the first `length` bytes are the vector. */
	data: Uint8Array;
	length = 0;

	constructor(capacity = 0) {
		this.data = bytes(capacity);
	}
}

/** A `Vec<u8>` of a copy of `bytes`. */
export function byteVecFrom(bytes: Uint8Array): ByteVec {
	const v = new ByteVec(bytes.length);
	v.data.set(bytes);
	v.length = bytes.length;
	return v;
}

/** `vec![value; n]` of bytes. */
export function byteVecOf(value: number, n: number): ByteVec {
	const v = new ByteVec(n);
	v.data.fill(value);
	v.length = n;
	return v;
}

/** Make room for `additional` more bytes. */
function reserve(v: ByteVec, additional: number): void {
	const needed = v.length + additional;
	if (needed <= v.data.length) return;
	const grown = bytes(Math.max(needed, v.data.length * 2, 8));
	// All of `data`, which costs less than making a view of its first `length` bytes.
	grown.set(v.data);
	v.data = grown;
}

/** The bytes, as a view that a later push may detach. */
export function byteVecView(v: ByteVec): Uint8Array {
	return v.data.subarray(0, v.length);
}

/** The bytes, as `Vec`'s `Deref` borrows them, which a later push may detach. */
export function byteVecSlice(v: ByteVec): ByteSlice {
	return new ByteSlice(v.data, 0, v.length);
}

/** `Vec::push`. */
export function byteVecPush(v: ByteVec, byte: number): void {
	reserve(v, 1);
	v.data[v.length++] = byte;
}

/** `Vec::pop`: the last byte, or `undefined` if empty. */
export function byteVecPop(v: ByteVec): number | undefined {
	return v.length === 0 ? undefined : v.data[--v.length];
}

/** `Vec::extend_from_slice`. */
export function byteVecExtendFromSlice(v: ByteVec, s: ByteSlice): void {
	reserve(v, s.length);
	copyBytes(v.data, v.length, s.data, s.start, s.length);
	v.length += s.length;
}

/** `Vec::truncate`. */
export function byteVecTruncate(v: ByteVec, length: number): void {
	if (length < v.length) v.length = length;
}

/** `Vec::clear`. */
export function byteVecClear(v: ByteVec): void {
	v.length = 0;
}

// Strings

const encoder = /* @__PURE__ */ new TextEncoder();
// `ignoreBOM` keeps a leading U+FEFF, as Rust does.
const decoder = /* @__PURE__ */ new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });

/** `alloc::string::FromUtf8Error`: the bytes that were not UTF-8, and where they stop being. */
export class FromUtf8Error {
	readonly bytes: ByteVec;
	/** `Utf8Error::valid_up_to`. */
	readonly validUpTo: number;
	/** `Utf8Error::error_len`: `undefined` if the input ended mid-character. */
	readonly errorLen: number | undefined;

	constructor(bytes: ByteVec) {
		this.bytes = bytes;
		[this.validUpTo, this.errorLen] = utf8Error(byteVecView(bytes));
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

/** `String::from_utf8`. */
export function stringFromUtf8(bytes: ByteVec): Result<string, FromUtf8Error> {
	try {
		return Ok(decoder.decode(byteVecView(bytes)));
	} catch {
		return Err(new FromUtf8Error(bytes));
	}
}

/** `str::as_bytes`: its UTF-8 bytes. */
export function stringAsBytes(s: string): ByteSlice {
	return byteSliceFrom(encoder.encode(s));
}

/** `str::len`: its length in UTF-8 bytes. */
export function stringLen(s: string): number {
	return encoder.encode(s).length;
}

/** `char::from_u32`: `n`, unless it is a surrogate or past the last char. */
export function charFromU32(n: u32): char | undefined {
	return n > 0x10ffff || (n >= 0xd800 && n <= 0xdfff) ? undefined : (n as char);
}

/** `char::is_ascii_digit`. */
export function charIsAsciiDigit(c: char): boolean {
	return c >= 0x30 && c <= 0x39;
}

// Formatting, as `core::fmt` does it: the same output byte for byte.

/** `core::fmt::Error`. */
export class FmtError {}

/** `fmt::Result`. */
export type FmtResult = Result<undefined, FmtError>;

const written: FmtResult = /* @__PURE__ */ Ok(undefined);

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

/** `core::fmt::Formatter`: where it writes, and its placeholder's options. */
export interface Formatter {
	readonly out: Write;
	readonly spec: Spec;
}

/** `fmt::write`: each placeholder gets a fresh `Formatter` with only its own options. */
function write(out: Write, args: Arguments): FmtResult {
	for (const part of args) {
		const r = typeof part === "string" ? out.writeStr(part) : part({ out, spec: {} });
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

/** `Formatter::write_str`. */
export function fmtWriteStr(f: Formatter, s: string): FmtResult {
	return f.out.writeStr(s);
}

/** `Formatter::write_fmt`. */
export function fmtWriteFmt(f: Formatter, args: Arguments): FmtResult {
	return write(f.out, args);
}

/** `Formatter::alternate`: whether the spec has `#`. */
export function fmtAlternate(f: Formatter): boolean {
	return f.spec.alternate === true;
}

/** `Formatter::width`: the spec's width, or `undefined`. */
export function fmtWidth(f: Formatter): number | undefined {
	return f.spec.width;
}

/** Write `s`, padded to the width. */
export function fmtPad(f: Formatter, s: string): FmtResult {
	const width = f.spec.width;
	const count = width === undefined ? 0 : chars(s);
	if (width === undefined || count >= width) return f.out.writeStr(s);
	return padded(f, width - count, f.spec.align ?? "<", f.spec.fill ?? " ", s);
}

/** Write an integer's sign, `prefix` if alternate, and `digits`, padded to the width. */
export function fmtPadIntegral(f: Formatter, nonnegative: boolean, prefix: string, digits: string): FmtResult {
	const head = (nonnegative ? "" : "-") + (fmtAlternate(f) ? prefix : "");
	const length = head.length + digits.length;
	const width = f.spec.width ?? 0;
	if (length >= width) return f.out.writeStr(head + digits);
	if (f.spec.zero) {
		const r = f.out.writeStr(head);
		return r.ok ? padded(f, width - length, ">", "0", digits) : r;
	}
	return padded(f, width - length, f.spec.align ?? ">", f.spec.fill ?? " ", head + digits);
}

/** Write `s` with `padding` fills around it. */
function padded(f: Formatter, padding: number, align: "<" | "^" | ">", fill: string, s: string): FmtResult {
	const left = align === "<" ? 0 : align === ">" ? padding : Math.floor(padding / 2);
	const r = f.out.writeStr(fill.repeat(left) + s);
	return r.ok ? f.out.writeStr(fill.repeat(padding - left)) : r;
}

/** `Formatter::debug_struct`. */
export function fmtDebugStruct(f: Formatter, name: string): DebugStruct {
	return { fmt: f, result: f.out.writeStr(name), hasFields: false };
}

/** `Formatter::debug_tuple`. */
export function fmtDebugTuple(f: Formatter, name: string): DebugTuple {
	return { fmt: f, result: f.out.writeStr(name), fields: 0, emptyName: name === "" };
}

/** `Formatter::debug_list`. */
export function fmtDebugList(f: Formatter): DebugList {
	return { fmt: f, result: f.out.writeStr("["), hasFields: false };
}

/** `debug_struct_fields_finish`, which derived `Debug` calls past five fields. */
export function fmtDebugStructFields(f: Formatter, name: string, names: readonly string[], values: readonly Fmt[]): FmtResult {
	if (names.length !== values.length) {
		assertFailed(
			"==",
			(f) => usizeDisplay(names.length, f),
			(f) => usizeDisplay(values.length, f),
		);
	}
	const builder = fmtDebugStruct(f, name);
	names.forEach((field, i) => debugStructField(builder, field, values[i]));
	return debugStructFinish(builder);
}

/** `debug_tuple_fields_finish`, which derived `Debug` calls past five fields. */
export function fmtDebugTupleFields(f: Formatter, name: string, values: readonly Fmt[]): FmtResult {
	const builder = fmtDebugTuple(f, name);
	for (const value of values) debugTupleField(builder, value);
	return debugTupleFinish(builder);
}

/** `fmt::DebugStruct`. */
export interface DebugStruct {
	readonly fmt: Formatter;
	result: FmtResult;
	hasFields: boolean;
}

/** `DebugStruct::field`. */
export function debugStructField(b: DebugStruct, name: string, value: Fmt): DebugStruct {
	if (b.result.ok) {
		if (fmtAlternate(b.fmt)) {
			const head = b.hasFields ? written : b.fmt.out.writeStr(" {\n");
			b.result = head.ok ? pretty(b.fmt, [name, ": ", value, ",\n"]) : head;
		} else {
			b.result = b.fmt.out.writeStr((b.hasFields ? ", " : " { ") + name + ": ");
			if (b.result.ok) b.result = value(b.fmt);
		}
	}
	b.hasFields = true;
	return b;
}

/** `DebugStruct::finish_non_exhaustive`. */
export function debugStructFinishNonExhaustive(b: DebugStruct): FmtResult {
	if (b.result.ok) {
		if (!b.hasFields) b.result = b.fmt.out.writeStr(" { .. }");
		else if (!fmtAlternate(b.fmt)) b.result = b.fmt.out.writeStr(", .. }");
		else b.result = closePretty(b.fmt, "}");
	}
	return b.result;
}

/** `DebugStruct::finish`. */
export function debugStructFinish(b: DebugStruct): FmtResult {
	if (b.hasFields && b.result.ok) b.result = b.fmt.out.writeStr(fmtAlternate(b.fmt) ? "}" : " }");
	return b.result;
}

/** `fmt::DebugTuple`. */
export interface DebugTuple {
	readonly fmt: Formatter;
	result: FmtResult;
	fields: number;
	readonly emptyName: boolean;
}

/** `DebugTuple::field`. */
export function debugTupleField(b: DebugTuple, value: Fmt): DebugTuple {
	if (b.result.ok) {
		if (fmtAlternate(b.fmt)) {
			const head = b.fields > 0 ? written : b.fmt.out.writeStr("(\n");
			b.result = head.ok ? pretty(b.fmt, [value, ",\n"]) : head;
		} else {
			b.result = b.fmt.out.writeStr(b.fields === 0 ? "(" : ", ");
			if (b.result.ok) b.result = value(b.fmt);
		}
	}
	b.fields++;
	return b;
}

/** `DebugTuple::finish_non_exhaustive`. */
export function debugTupleFinishNonExhaustive(b: DebugTuple): FmtResult {
	if (b.result.ok) {
		if (b.fields === 0) b.result = b.fmt.out.writeStr("(..)");
		else if (!fmtAlternate(b.fmt)) b.result = b.fmt.out.writeStr(", ..)");
		else b.result = closePretty(b.fmt, ")");
	}
	return b.result;
}

/** `DebugTuple::finish`. */
export function debugTupleFinish(b: DebugTuple): FmtResult {
	if (b.fields > 0 && b.result.ok) {
		const comma = b.fields === 1 && b.emptyName && !fmtAlternate(b.fmt);
		b.result = b.fmt.out.writeStr(comma ? ",)" : ")");
	}
	return b.result;
}

/** `fmt::DebugList`. */
export interface DebugList {
	readonly fmt: Formatter;
	result: FmtResult;
	hasFields: boolean;
}

/** `DebugList::entry`. */
export function debugListEntry(b: DebugList, value: Fmt): DebugList {
	if (b.result.ok) {
		if (fmtAlternate(b.fmt)) {
			const head = b.hasFields ? written : b.fmt.out.writeStr("\n");
			b.result = head.ok ? pretty(b.fmt, [value, ",\n"]) : head;
		} else {
			b.result = b.hasFields ? b.fmt.out.writeStr(", ") : written;
			if (b.result.ok) b.result = value(b.fmt);
		}
	}
	b.hasFields = true;
	return b;
}

/** `DebugList::entries`. */
export function debugListEntries<T>(b: DebugList, items: Iterable<T>, debug: Debug<T>): DebugList {
	for (const item of items) debugListEntry(b, (f) => debug(item, f));
	return b;
}

/** `DebugList::finish_non_exhaustive`. */
export function debugListFinishNonExhaustive(b: DebugList): FmtResult {
	if (!b.result.ok) return b.result;
	if (!b.hasFields) return b.fmt.out.writeStr("..]");
	if (!fmtAlternate(b.fmt)) return b.fmt.out.writeStr(", ..]");
	return closePretty(b.fmt, "]");
}

/** `DebugList::finish`. */
export function debugListFinish(b: DebugList): FmtResult {
	if (b.result.ok) b.result = b.fmt.out.writeStr("]");
	return b.result;
}

/** Write `parts` indented, as a pretty builder writes one field: values keep `f`'s options. */
function pretty(f: Formatter, parts: Arguments): FmtResult {
	const writer: Formatter = { out: new PadAdapter(f.out), spec: f.spec };
	for (const part of parts) {
		const r = typeof part === "string" ? writer.out.writeStr(part) : part(writer);
		if (!r.ok) return r;
	}
	return written;
}

/** A pretty builder's `finish_non_exhaustive`: an indented `..`, then `close`. */
function closePretty(f: Formatter, close: string): FmtResult {
	const r = pretty(f, ["..\n"]);
	return r.ok ? f.out.writeStr(close) : r;
}

// `Debug` and `Display` of what std defines them for. A string's `Display` is `fmtPad`.

/** Characters `Debug` of a string writes as `\u{..}`, by std's rule. */
const unprintable = /[\p{Cc}\p{Co}\p{White_Space}\p{Grapheme_Extend}\p{Default_Ignorable_Code_Point}\p{Cf}\P{Assigned}]/u;

/** One char of `Debug` of a string, which escapes `"`, or of a char, which escapes `'`. */
function escapeDebug(c: string, quote: '"' | "'"): string {
	switch (c) {
		case quote:
			return `\\${quote}`;
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

/** `format!`, from `alloc::fmt::format`. */
export function fmtFormat(args: Arguments): string {
	const out = new StringWriter();
	if (!write(out, args).ok) {
		panic("a formatting trait implementation returned an error when the underlying stream did not: Error");
	}
	return out.s;
}

/** `ToString::to_string` of a `Display` value. */
export function fmtToString(value: Fmt): string {
	const out = new StringWriter();
	if (!value({ out, spec: {} }).ok) panic("a Display implementation returned an error unexpectedly: Error");
	return out.s;
}

/** A placeholder with options, which replace the defaults its value is written with. */
export function fmtSpec(spec: Spec, value: Fmt): Fmt {
	return (f) => value({ out: f.out, spec });
}

/** `Display` and `Debug` of a `bool`. */
export function fmtBool(value: boolean, f: Formatter): FmtResult {
	return fmtPad(f, value ? "true" : "false");
}

/** `Debug` of a string. */
export function fmtStr(s: string, f: Formatter): FmtResult {
	let out = '"';
	for (const c of s) out += escapeDebug(c, '"');
	return f.out.writeStr(out + '"');
}

/** `Display` of a `char`. */
export function fmtCharDisplay(c: char, f: Formatter): FmtResult {
	return fmtPad(f, String.fromCodePoint(c));
}

/** `Debug` of a `char`, which no width pads. */
export function fmtCharDebug(c: char, f: Formatter): FmtResult {
	return f.out.writeStr(`'${escapeDebug(String.fromCodePoint(c), "'")}'`);
}

/** `Debug` of `()`. */
export function fmtUnit(_: undefined, f: Formatter): FmtResult {
	return fmtPad(f, "()");
}

/** `Debug` of an `Option`, through its value's. */
export function fmtOption<T>(o: T | undefined, f: Formatter, debug: Debug<T>): FmtResult {
	if (o === undefined) return f.out.writeStr("None");
	return debugTupleFinish(debugTupleField(fmtDebugTuple(f, "Some"), (f) => debug(o, f)));
}

/** `Debug` of a `Result`, through its value's and error's. */
export function fmtResult<T, E>(r: Result<T, E>, f: Formatter, ok: Debug<T>, err: Debug<E>): FmtResult {
	const field: Fmt = r.ok ? (f) => ok(r.value, f) : (f) => err(r.error, f);
	return debugTupleFinish(debugTupleField(fmtDebugTuple(f, r.ok ? "Ok" : "Err"), field));
}

/** `Debug` of a slice, array, or `Vec`. */
export function fmtList<T>(items: Iterable<T>, f: Formatter, debug: Debug<T>): FmtResult {
	return debugListFinish(debugListEntries(fmtDebugList(f), items, debug));
}

/** `Debug` of a tuple, given each element's. */
export function fmtTuple(fields: readonly Fmt[], f: Formatter): FmtResult {
	const builder = fmtDebugTuple(f, "");
	for (const field of fields) debugTupleField(builder, field);
	return debugTupleFinish(builder);
}

/** `Display` of `fmt::Arguments`, which ignores the formatter's options. */
export function fmtArguments(args: Arguments, f: Formatter): FmtResult {
	return fmtWriteFmt(f, args);
}

/** `Debug` of `fmt::Error`. */
export function fmtErrorDebug(_: FmtError, f: Formatter): FmtResult {
	return f.out.writeStr("Error");
}

/** `Display` of `fmt::Error`. */
export function fmtErrorDisplay(_: FmtError, f: Formatter): FmtResult {
	return fmtPad(f, "an error occurred when formatting an argument");
}

/** `Debug` of `TryFromIntError`. */
export function fmtTryFromIntErrorDebug(e: TryFromIntError, f: Formatter): FmtResult {
	return debugTupleFinish(debugTupleField(fmtDebugTuple(f, "TryFromIntError"), (f) => f.out.writeStr(e.kind)));
}

/** `Display` of `TryFromIntError`. */
export function fmtTryFromIntErrorDisplay(e: TryFromIntError, f: Formatter): FmtResult {
	const positive = e.kind === "PosOverflow";
	return fmtPad(f, `number too ${positive ? "large" : "small"} to fit in target type`);
}

/** `Debug` of `FromUtf8Error`. */
export function fmtFromUtf8ErrorDebug(e: FromUtf8Error, f: Formatter): FmtResult {
	const error: Fmt = (f) =>
		debugStructFinish(
			debugStructField(
				debugStructField(fmtDebugStruct(f, "Utf8Error"), "valid_up_to", (f) => usizeDisplay(e.validUpTo, f)),
				"error_len",
				(f) => fmtOption(e.errorLen, f, u8Display),
			),
		);
	return debugStructFinish(
		debugStructField(
			debugStructField(fmtDebugStruct(f, "FromUtf8Error"), "bytes", (f) => fmtList(byteVecView(e.bytes), f, u8Display)),
			"error",
			error,
		),
	);
}

/** `Display` of the error, which `write!`s and so ignores the formatter's options. */
export function fmtFromUtf8ErrorDisplay(e: FromUtf8Error, f: Formatter): FmtResult {
	if (e.errorLen === undefined) return f.out.writeStr(`incomplete utf-8 byte sequence from index ${e.validUpTo}`);
	return f.out.writeStr(`invalid utf-8 sequence of ${e.errorLen} bytes from index ${e.validUpTo}`);
}

