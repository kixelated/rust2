// The runtime of rust2-ts output: what Rust's std does that plain JS does not.
//
// Every operation is its own exported function, named by what it acts on, `sliceRange`
// or `fmtPad`, so a bundler drops every one translated code never calls. Methods are
// left only where a call dispatches on its receiver: an iterator's `next`, a writer's
// `writeStr`, a future's `poll`, a waker's, and `[Symbol.dispose]`.

import type { Ordering } from "./rt/collections.ts";
import { type char, type TryFromIntError, type u32, type usize, u8Debug, usizeDebug, usizeDisplay } from "./rt/int.ts";
import { dropAll, type Pending } from "./rt/task.ts";

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

// Bindings

/**
 * `T` if it can be a bound Rust type's value, or else why not: never `undefined` or
 * `Pending`, which `None` and `Poll::Pending` are, an object unless Rust copies it, and
 * disposable exactly when it drops.
 */
export type Bound<T, Copy extends boolean, Drop extends boolean> = [undefined] extends [T]
	? "a bound value can't be undefined, which None is"
	: [typeof Pending] extends [T]
		? "a bound value can't be Pending, which Poll::Pending is"
		: [Copy, T] extends [false, object]
			? Disposes<T, Drop>
			: Copy extends true
				? Disposes<T, Drop>
				: "a bound value Rust doesn't copy must be an object, which a reference changes in place";

/** `T` if it has a `[Symbol.dispose]` exactly when `Drop`, or else why not. */
type Disposes<T, Drop extends boolean> = Drop extends true
	? [T] extends [Disposable]
		? T
		: "a bound value that drops must have a [Symbol.dispose]"
	: [T] extends [{ [Symbol.dispose]: unknown }]
		? "a bound value with a [Symbol.dispose] must bind with drop = true"
		: T;

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

/** `Ok(())`, one frozen object every evaluation shares, as nothing can change it. */
export const OK: Result<undefined, never> = /* @__PURE__ */ Object.freeze({ ok: true, value: undefined });

/** `Err(())`, shared as `OK` is. */
export const ERR: Result<never, undefined> = /* @__PURE__ */ Object.freeze({ ok: false, error: undefined });

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

// Options, which are `T | undefined`, or `T | None` where `T` can already be `undefined`.
// A function taking one takes that `None` after its own arguments.

/** `None` of an `Option` whose `Some` can already be `undefined`, as `Option<()>`'s is. */
export const None: unique symbol = /* @__PURE__ */ Symbol("None");

/** The type of `None`. */
export type None = typeof None;

/** `Option::unwrap`: the value, or a panic. */
export function optionUnwrap<T>(o: T | undefined): T;
export function optionUnwrap<T>(o: T | None, none: None): T;
export function optionUnwrap<T>(o: T | undefined | None, none?: None): T {
	if (o === none) panic("called `Option::unwrap()` on a `None` value");
	return o as T;
}

/** `Option::expect`: the value, or a panic with `message`. */
export function optionExpect<T>(o: T | undefined, message: string): T;
export function optionExpect<T>(o: T | None, message: string, none: None): T;
export function optionExpect<T>(o: T | undefined | None, message: string, none?: None): T {
	if (o === none) panic(message);
	return o as T;
}

/** `bool::then_some`: `value` if `cond`, which is evaluated either way, and else drops. */
export function thenSome<T>(cond: boolean, value: T, drop?: (value: T) => void): T | undefined;
export function thenSome<T>(cond: boolean, value: T, drop: ((value: T) => void) | undefined, none: None): T | None;
export function thenSome<T>(cond: boolean, value: T, drop?: (value: T) => void, none?: None): T | undefined | None {
	if (cond) return value;
	drop?.(value);
	return none;
}

// Not `then`, which would make the module's namespace a thenable that `await` calls.
/** `bool::then`: `f()` if `cond`, which is called only then, and else `f` drops. */
export function boolThen<T>(cond: boolean, f: () => T, drop?: (f: () => T) => void): T | undefined;
export function boolThen<T>(cond: boolean, f: () => T, drop: ((f: () => T) => void) | undefined, none: None): T | None;
export function boolThen<T>(cond: boolean, f: () => T, drop?: (f: () => T) => void, none?: None): T | undefined | None {
	if (cond) return f();
	drop?.(f);
	return none;
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
export function sliceFirst<T>(s: ArrayLike<T>): T | undefined;
export function sliceFirst<T>(s: ArrayLike<T>, none: None): T | None;
export function sliceFirst<T>(s: ArrayLike<T>, none?: None): T | undefined | None {
	return s.length === 0 ? none : s[0];
}

/** `get`: the element at `index`, or `undefined` past the end. */
export function sliceGet<T>(s: ArrayLike<T>, index: number): T | undefined;
export function sliceGet<T>(s: ArrayLike<T>, index: number, none: None): T | None;
export function sliceGet<T>(s: ArrayLike<T>, index: number, none?: None): T | undefined | None {
	return index < s.length ? s[index] : none;
}

/** `core::ops::Bound`, tagged as the crate's own enums are. */
export type RangeBound<T> = { readonly tag: "Included"; readonly _0: T } | { readonly tag: "Excluded"; readonly _0: T } | { readonly tag: "Unbounded" };

/** `Bound::cloned` and `copied`: `b` with `clone` of what it holds. */
export function boundMap<T>(b: RangeBound<T>, clone: (value: T) => T): RangeBound<T> {
	return b.tag === "Unbounded" ? b : { tag: b.tag, _0: clone(b._0) };
}

/** `NonZero::new`: `n`, unless it is 0. */
export function nonZero<T>(n: T): T | undefined {
	return n === 0 ? undefined : n;
}

/** `reverse`, in place. */
export function sliceReverse<T>(s: T[]): void {
	s.reverse();
}

/** `last` and `last_mut`: the last element, or `undefined` if empty. */
export function sliceLast<T>(s: ArrayLike<T>): T | undefined {
	return s.length === 0 ? undefined : s[s.length - 1];
}

/** `contains`, by `eq` of each element and then `x`, as std's `==` reads them. */
export function sliceContains<T>(s: ArrayLike<T>, x: T, eq: (a: T, b: T) => boolean): boolean {
	for (let i = 0; i < s.length; i++) if (eq(s[i], x)) return true;
	return false;
}

/** `swap`, panicking on either index past the end, `a` first, as std's indexing does. */
export function sliceSwap<T>(s: T[], a: number, b: number): void {
	const x = s[idx(s, a)];
	s[a] = s[idx(s, b)];
	s[b] = x;
}

/**
 * `sort` and `sort_by`, and the unstable sorts, which may order equal elements either way
 * and here keep them in order: stable, by `cmp`, which moves an element before another
 * only if it orders `Less`, as std's sorts test `lt`. If `cmp` panics, the slice holds each
 * element once, in an order std leaves unspecified, as std's does.
 */
export function sliceSortBy<T>(s: T[], cmp: (a: T, b: T) => Ordering | undefined): void {
	sort(s, (a, b) => cmp(a, b) === -1);
}

/** `sort_by_key` and `sort_unstable_by_key`: by `cmp` of each element's `key`, as `sliceSortBy` sorts. */
export function sliceSortByKey<T, K>(s: T[], key: (item: T) => K, cmp: (a: K, b: K) => Ordering | undefined): void {
	sort(s, (a, b) => cmp(key(a), key(b)) === -1);
}

/**
 * A stable merge sort of `s` by `less`, over runs that insertion sorts first. It calls only
 * `less`, never JS's `sort`, which moves `undefined`, a `None`, to the end without
 * comparing it, and orders its calls differently in each engine.
 */
function sort<T>(s: T[], less: (a: T, b: T) => boolean): void {
	const n = s.length;
	const RUN = 16;
	for (let lo = 0; lo < n; lo += RUN) {
		const hi = Math.min(lo + RUN, n);
		for (let i = lo + 1; i < hi; i++) {
			const x = s[i];
			let j = i;
			// Shifted elements leave a hole at `j`, which `x` fills if `less` panics.
			try {
				while (j > lo && less(x, s[j - 1])) {
					s[j] = s[j - 1];
					j--;
				}
			} finally {
				s[j] = x;
			}
		}
	}
	if (n <= RUN) return;
	const left: T[] = [];
	for (let width = RUN; width < n; width *= 2) {
		for (let lo = 0; lo + width < n; lo += 2 * width) {
			const mid = lo + width;
			const hi = Math.min(mid + width, n);
			if (!less(s[mid], s[mid - 1])) continue;
			// The left run moves aside, and the merge fills `s` from `lo`, never past the
			// right run's next element. If `less` panics, what is left of the left run fills
			// the gap.
			left.length = 0;
			for (let i = lo; i < mid; i++) left.push(s[i]);
			let i = 0;
			let j = mid;
			let k = lo;
			try {
				// `k` moves only once `less` returns, so a panic leaves no slot written twice.
				while (i < left.length && j < hi) {
					s[k] = less(s[j], left[i]) ? s[j++] : left[i++];
					k++;
				}
			} finally {
				while (i < left.length) s[k++] = left[i++];
			}
		}
	}
}

/**
 * `Vec::retain` and `retain_mut`: keeps each element `f` returns true for, in order, and
 * drops each other one once `f` rejects it. If `f` or a drop panics, what `f` kept and
 * what it had not yet seen stay, as std's do.
 */
export function vecRetain<T>(v: T[], f: (item: T) => boolean, drop?: (item: T) => void): void {
	const n = v.length;
	let read = 0;
	let write = 0;
	try {
		while (read < n) {
			const item = v[read];
			if (f(item)) {
				v[write++] = item;
				read++;
			} else {
				read++;
				drop?.(item);
			}
		}
	} finally {
		v.copyWithin(write, read, n);
		v.length = write + n - read;
	}
}

/**
 * `Vec::dedup_by`: drops each element `same(element, previous kept)` is true for, in order.
 * If `same` or a drop panics, what it kept and what it had not yet seen stay, as std's do.
 */
export function vecDedupBy<T>(v: T[], same: (a: T, b: T) => boolean, drop?: (item: T) => void): void {
	const n = v.length;
	let first = 1;
	while (first < n && !same(v[first], v[first - 1])) first++;
	if (first >= n) return;
	let read = first + 1;
	let write = first;
	try {
		drop?.(v[first]);
		while (read < n) {
			const item = v[read];
			if (same(item, v[write - 1])) {
				read++;
				drop?.(item);
			} else {
				v[write++] = item;
				read++;
			}
		}
	} finally {
		v.copyWithin(write, read, n);
		v.length = write + n - read;
	}
}

/** `Vec::dedup_by_key`: as `vecDedupBy`, by `eq` of `key` of each element, then of the previous kept. */
export function vecDedupByKey<T, K>(v: T[], key: (item: T) => K, eq: (a: K, b: K) => boolean, drop?: (item: T) => void): void {
	vecDedupBy(v, (a, b) => eq(key(a), key(b)), drop);
}

/** `Vec::insert`: `item` at `i`, with those from `i` on moved up. Past the end, it drops `item` and panics. */
export function vecInsert<T>(v: T[], i: number, item: T, drop?: (item: T) => void): void {
	if (i > v.length) {
		try {
			panic(`insertion index (is ${i}) should be <= len (is ${v.length})`);
		} finally {
			drop?.(item);
		}
	}
	v.splice(i, 0, item);
}

/** `Vec::remove`: the element at `i`, with those after it moved down. */
export function vecRemove<T>(v: T[], i: number): T {
	if (i >= v.length) panic(`removal index (is ${i}) should be < len (is ${v.length})`);
	return v.splice(i, 1)[0];
}

/**
 * `Vec::truncate`, and `clear` to 0: the first `len` elements, if there are more, and then
 * each other drops in order, the rest still if one panics.
 */
export function vecTruncate<T>(v: T[], len: number, drop?: (item: T) => void): void {
	if (len >= v.length) return;
	const cut = v.splice(len);
	if (drop !== undefined) dropAll(cut, drop);
}

/** `Vec::swap_remove`: the element at `i`, with the last moved into its place. */
export function vecSwapRemove<T>(v: T[], i: number): T {
	if (i >= v.length) panic(`swap_remove index (is ${i}) should be < len (is ${v.length})`);
	const item = v[i];
	v[i] = v[v.length - 1];
	v.pop();
	return item;
}

/**
 * `Clone` of a `Vec`, and `to_vec`, by `clone` of each element in order. If one panics, the
 * clones made so far drop, in order, as std's do.
 */
export function vecClone<T>(s: ArrayLike<T>, clone: (item: T) => T, drop?: (item: T) => void): T[] {
	const out: T[] = [];
	try {
		for (let i = 0; i < s.length; i++) out.push(clone(s[i]));
	} catch (e) {
		if (drop !== undefined) for (const item of out) drop(item);
		throw e;
	}
	return out;
}

/** `Vec::extend_from_slice`: a clone of each element of `s`, pushed in order, as far as they clone. */
export function vecExtendFromSlice<T>(v: T[], s: ArrayLike<T>, clone?: (item: T) => T): void {
	for (let i = 0; i < s.length; i++) v.push(clone === undefined ? s[i] : clone(s[i]));
}

/**
 * `Clone` of a tuple, by `clone` of each field in order, `undefined` for one whose value is
 * itself. If one panics, the clones made so far drop, the last first, by `drops`.
 */
export function tupleClone<T extends readonly unknown[]>(
	t: T,
	clones: readonly (((value: never) => unknown) | undefined)[],
	drops: readonly (((value: never) => void) | undefined)[],
): T {
	const out: unknown[] = [];
	try {
		for (let i = 0; i < clones.length; i++) {
			const clone = clones[i] as ((value: unknown) => unknown) | undefined;
			out.push(clone === undefined ? t[i] : clone(t[i]));
		}
	} catch (e) {
		for (let i = out.length - 1; i >= 0; i--) (drops[i] as ((value: unknown) => void) | undefined)?.(out[i]);
		throw e;
	}
	return out as unknown as T;
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

/** `core::str::Utf8Error`: where bytes stop being UTF-8. */
export type Utf8Error = {
	/** `Utf8Error::valid_up_to`. */
	readonly validUpTo: usize;
	/** `Utf8Error::error_len`: `undefined` if the input ended mid-character. */
	readonly errorLen: usize | undefined;
};

/** `alloc::string::FromUtf8Error`: the bytes that were not UTF-8, and where they stop being. */
export class FromUtf8Error {
	readonly bytes: ByteVec;
	readonly error: Utf8Error;

	constructor(bytes: ByteVec) {
		this.bytes = bytes;
		this.error = utf8Error(byteVecView(bytes));
	}
}

/** `Clone` of a `FromUtf8Error`: its bytes, copied. */
export function fromUtf8ErrorClone(e: FromUtf8Error): FromUtf8Error {
	return new FromUtf8Error(byteVecFrom(byteVecView(e.bytes)));
}

/** Where `v`, which is not UTF-8, stops being so, and how many bytes are invalid there, as std reports it. */
function utf8Error(v: Uint8Array): Utf8Error {
	const tail = (b: number | undefined) => b !== undefined && b >= 0x80 && b <= 0xbf;
	for (let i = 0; i < v.length; ) {
		const first = v[i];
		const width = first < 0x80 ? 1 : first >= 0xc2 && first <= 0xdf ? 2 : first >= 0xe0 && first <= 0xef ? 3 : first >= 0xf0 && first <= 0xf4 ? 4 : 0;
		if (width === 1) {
			i++;
			continue;
		}
		if (width === 0) return { validUpTo: i, errorLen: 1 };
		const second = v[i + 1];
		if (second === undefined) return { validUpTo: i, errorLen: undefined };
		const [lo, hi] =
			first === 0xe0 ? [0xa0, 0xbf] : first === 0xed ? [0x80, 0x9f] : first === 0xf0 ? [0x90, 0xbf] : first === 0xf4 ? [0x80, 0x8f] : [0x80, 0xbf];
		if (second < lo || second > hi) return { validUpTo: i, errorLen: 1 };
		for (let k = 2; k < width; k++) {
			if (i + k >= v.length) return { validUpTo: i, errorLen: undefined };
			if (!tail(v[i + k])) return { validUpTo: i, errorLen: k };
		}
		i += width;
	}
	throw new Error("utf8Error: the bytes are valid UTF-8");
}

/** `str::from_utf8`. */
export function strFromUtf8(bytes: ByteSlice): Result<string, Utf8Error> {
	const v = byteSliceView(bytes);
	try {
		return Ok(decoder.decode(v));
	} catch {
		return Err(utf8Error(v));
	}
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
	/** The `+` flag: a sign on a nonnegative number too. */
	readonly plus?: boolean;
	/** The `#` flag: `{:#?}` and `{:#x}`. */
	readonly alternate?: boolean;
	/** The `0` flag: zeros between the sign and the digits. */
	readonly zero?: boolean;
	/** `{:x?}` or `{:X?}`: an integer's `Debug` in hex. */
	readonly debugHex?: "x" | "X";
	readonly width?: number;
	/** The `.N` of `{:.N}`: the most chars `pad` writes, or digits past a number's point. */
	readonly precision?: number;
}

/** A width or precision from an argument, as in `{:1$}`: a `fmt::rt::Argument` only a spec reads. */
export interface Count extends Fmt {
	readonly count: number;
}

/** `Argument::from_usize`: `n` as a width or precision, panicking past what one holds. */
export function fmtCount(n: number): Count {
	if (n > 0xffff) panic("Formatting argument out of range");
	const unreachable = (): never => {
		throw new Error("rust2-ts: a width or precision written as a placeholder");
	};
	return Object.assign(unreachable, { count: n });
}

/** The width or precision `argument`, a `Count`, holds. */
export function fmtCountOf(argument: Fmt): number {
	return (argument as Count).count;
}

/** `dyn fmt::Write`: where a `Formatter` writes. */
export interface Write {
	writeStr(s: string): FmtResult;
}

class StringWriter implements Write {
	s = "";

	writeStr(s: string): FmtResult {
		this.s += s;
		return OK;
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
		return OK;
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
	return OK;
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

/** Write `s`, cut to the precision in chars, padded to the width. */
export function fmtPad(f: Formatter, s: string): FmtResult {
	const { width, precision } = f.spec;
	if (width === undefined && precision === undefined) return f.out.writeStr(s);
	let count = 0;
	if (precision === undefined) {
		count = chars(s);
	} else {
		let end = 0;
		for (const c of s) {
			if (count === precision) break;
			end += c.length;
			count++;
		}
		s = s.slice(0, end);
	}
	if (width === undefined || count >= width) return f.out.writeStr(s);
	return padded(f, width - count, f.spec.align ?? "<", f.spec.fill ?? " ", s);
}

/** Write an integer's sign, `prefix` if alternate, and `digits`, padded to the width. */
export function fmtPadIntegral(f: Formatter, nonnegative: boolean, prefix: string, digits: string): FmtResult {
	const head = (nonnegative ? (f.spec.plus ? "+" : "") : "-") + (fmtAlternate(f) ? prefix : "");
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
			const head = b.hasFields ? OK : b.fmt.out.writeStr(" {\n");
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
			const head = b.fields > 0 ? OK : b.fmt.out.writeStr("(\n");
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
			const head = b.hasFields ? OK : b.fmt.out.writeStr("\n");
			b.result = head.ok ? pretty(b.fmt, [value, ",\n"]) : head;
		} else {
			b.result = b.hasFields ? b.fmt.out.writeStr(", ") : OK;
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
	return OK;
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
export function fmtOption<T>(o: T | undefined, f: Formatter, debug: Debug<T>): FmtResult;
export function fmtOption<T>(o: T | None, f: Formatter, debug: Debug<T>, none: None): FmtResult;
export function fmtOption<T>(o: T | undefined | None, f: Formatter, debug: Debug<T>, none?: None): FmtResult {
	if (o === none) return f.out.writeStr("None");
	return debugTupleFinish(debugTupleField(fmtDebugTuple(f, "Some"), (f) => debug(o as T, f)));
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

/** `Debug` of `Utf8Error`. */
export function fmtUtf8ErrorDebug(e: Utf8Error, f: Formatter): FmtResult {
	return debugStructFinish(
		debugStructField(
			debugStructField(fmtDebugStruct(f, "Utf8Error"), "valid_up_to", (f) => usizeDebug(e.validUpTo, f)),
			"error_len",
			(f) => fmtOption(e.errorLen, f, usizeDebug),
		),
	);
}

/** `Display` of `Utf8Error`, which `write!`s and so ignores the formatter's options. */
export function fmtUtf8ErrorDisplay(e: Utf8Error, f: Formatter): FmtResult {
	if (e.errorLen === undefined) return f.out.writeStr(`incomplete utf-8 byte sequence from index ${e.validUpTo}`);
	return f.out.writeStr(`invalid utf-8 sequence of ${e.errorLen} bytes from index ${e.validUpTo}`);
}

/** `Debug` of `FromUtf8Error`. */
export function fmtFromUtf8ErrorDebug(e: FromUtf8Error, f: Formatter): FmtResult {
	return debugStructFinish(
		debugStructField(
			debugStructField(fmtDebugStruct(f, "FromUtf8Error"), "bytes", (f) => fmtList(byteVecView(e.bytes), f, u8Debug)),
			"error",
			(f) => fmtUtf8ErrorDebug(e.error, f),
		),
	);
}

/** `Display` of `FromUtf8Error`: its `Utf8Error`'s. */
export function fmtFromUtf8ErrorDisplay(e: FromUtf8Error, f: Formatter): FmtResult {
	return fmtUtf8ErrorDisplay(e.error, f);
}

