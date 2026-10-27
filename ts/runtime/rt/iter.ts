// Iterators, for rust2-ts output: std's `Iterator`, its adaptors, and `Range`, lazy as
// Rust's are. `rt.ts` re-exports all of it.
//
// `next` returns an item or `DONE`, never `undefined`, so an iterator of `Option<T>` keeps
// its `None` items apart from its end. Each adaptor calls what it wraps, and the closures
// it was given, in the order and as many times as std's does, and stops where std's
// stops: a `find` or `any` reads no further than its match. Adaptors and consumers are
// functions, `iterMap(it, f)`, so a bundler drops the ones nothing calls; an iterator's
// methods are only what each kind does its own way.

import { type Bound, ByteVec, byteVecPush, Err, Ok, panic, type Result } from "../rt.ts";
import { type Limbs, lt as wideLt, type Step } from "./int.ts";

/** What `next` returns once an iterator is exhausted, which no item is. */
export const DONE: unique symbol = /* @__PURE__ */ Symbol("DONE");

/** The type of `DONE`. */
export type Done = typeof DONE;

/**
 * What only the iterators std implements these traits for have, which Rust calls only
 * there: declared, not stubbed, so no bundle carries a stub.
 */
export interface Iter<T> {
	/** `DoubleEndedIterator::next_back`. */
	nextBack(): T | Done;
	/** `ExactSizeIterator::len`. */
	len(): number;
}

/** `Iterator`: `next`, and the methods std's iterators each do their own way. */
export abstract class Iter<T> implements Iterable<T> {
	/** The next item, or `DONE`. */
	abstract next(): T | Done;

	/** `nth`, as `next` returns it. */
	nthItem(n: number): T | Done {
		for (let i = 0; i < n; i++) if (this.next() === DONE) return DONE;
		return this.next();
	}

	/** `nth_back`, as `next_back` returns it. */
	nthBackItem(n: number): T | Done {
		for (let i = 0; i < n; i++) if (this.nextBack() === DONE) return DONE;
		return this.nextBack();
	}

	/** Its items, for a JS `for ... of`. */
	*[Symbol.iterator](): Iterator<T> {
		for (let item = this.next(); item !== DONE; item = this.next()) yield item;
	}
}

/** The item `next` returned, as an `Option`. */
export function iterToOption<T>(item: T | Done): T | undefined {
	return item === DONE ? undefined : item;
}

/** An `Option`, as `next` returns it. */
export function iterFromOption<T>(option: T | undefined): T | Done {
	return option === undefined ? DONE : option;
}

// Where std's iterators come from.

/** `slice::Iter`, and what iterating a `Vec` or array by value makes. */
export function iterSlice<T>(s: ArrayLike<T>): Iter<T> {
	return new SliceIter(s);
}

/** `Option::iter`, and `option::IntoIter`. */
export function iterOption<T>(o: T | undefined): Iter<T> {
	return new OptionIter(o);
}

/** `iter::once`. */
export function iterOnce<T>(value: T): Iter<T> {
	return new OptionIter<T>(value, true);
}

/** `iter::empty`. */
export function iterEmpty<T>(): Iter<T> {
	return new OptionIter<T>(undefined, false);
}

/** `iter::from_fn`, whose closure returns an `Option`. */
export function iterFromFn<T>(f: () => T | undefined): Iter<T> {
	return new FromNext(f, (f) => iterFromOption(f()));
}

/** `iter::successors`: `first`, then `succ` of each item, called as that item is yielded. */
export function iterSuccessors<T>(first: T | undefined, succ: (item: T) => T | undefined): Iter<T> {
	let next = first;
	return iterFromFn(() => {
		const item = next;
		if (item === undefined) return undefined;
		// Taken before `succ` runs, so a panic in it leaves the iterator done, as std's.
		next = undefined;
		next = succ(item);
		return item;
	});
}

/** An `impl Iterator` of the translated crate, which `next` calls the `next` of. */
export function iterFrom<S, T>(source: S, next: (source: S) => T | Done): Iter<T> {
	return new FromNext(source, next);
}

/** `Extend::extend` of a `Vec`. */
export function iterExtend<T>(v: T[], items: Iter<T>): void {
	for (let item = items.next(); item !== DONE; item = items.next()) v.push(item);
}

/** `Extend::extend` of a `Vec<u8>`. */
export function iterExtendBytes(v: ByteVec, items: Iter<number>): void {
	for (let item = items.next(); item !== DONE; item = items.next()) byteVecPush(v, item);
}

// Adaptors

/** `Iterator::map`. */
export function iterMap<T, U>(it: Iter<T>, f: (item: T) => U): Iter<U> {
	return new MapIter(it, f);
}

/** `Iterator::filter`. */
export function iterFilter<T>(it: Iter<T>, f: (item: T) => boolean): Iter<T> {
	return new FilterIter(it, f);
}

/** `Iterator::filter_map`. */
export function iterFilterMap<T, U>(it: Iter<T>, f: (item: T) => U | undefined): Iter<U> {
	return new FilterMapIter(it, f);
}

/** `Iterator::enumerate`. */
export function iterEnumerate<T>(it: Iter<T>): Iter<[number, T]> {
	return new EnumerateIter(it);
}

/** `chain`, with `other` already made an iterator, as std does when it is called. */
export function iterChain<T>(it: Iter<T>, other: Iter<T>): Iter<T> {
	return new ChainIter(it, other);
}

/**
 * `zip`, with `other` already made an iterator, as std does when it is called. A
 * `trusted` zip is one of two `TrustedRandomAccess` iterators, which std reads by
 * index up to the shorter length and never past it.
 */
export function iterZip<T, U>(it: Iter<T>, other: Iter<U>, trusted = false): Iter<[T, U]> {
	return trusted ? new TrustedZipIter(it, other) : new ZipIter(it, other);
}

/** `Iterator::rev`. */
export function iterRev<T>(it: Iter<T>): Iter<T> {
	return new RevIter(it);
}

/** `Iterator::skip`. */
export function iterSkip<T>(it: Iter<T>, n: number): Iter<T> {
	return new SkipIter(it, n);
}

/** `Iterator::take`. */
export function iterTake<T>(it: Iter<T>, n: number): Iter<T> {
	return new TakeIter(it, n);
}

/** `Iterator::skip_while`. */
export function iterSkipWhile<T>(it: Iter<T>, f: (item: T) => boolean): Iter<T> {
	return new SkipWhileIter(it, f);
}

/** `Iterator::take_while`. */
export function iterTakeWhile<T>(it: Iter<T>, f: (item: T) => boolean): Iter<T> {
	return new TakeWhileIter(it, f);
}

/** `Iterator::map_while`. */
export function iterMapWhile<T, U>(it: Iter<T>, f: (item: T) => U | undefined): Iter<U> {
	return new MapWhileIter(it, f);
}

/** `Iterator::step_by`, panicking on a step of 0. */
export function iterStepBy<T>(it: Iter<T>, step: number): Iter<T> {
	return new StepByIter(it, step);
}

/** `Iterator::inspect`. */
export function iterInspect<T>(it: Iter<T>, f: (item: T) => void): Iter<T> {
	return new InspectIter(it, f);
}

/** `Iterator::fuse`. */
export function iterFuse<T>(it: Iter<T>): Iter<T> {
	return new FuseIter(it);
}

/** `Iterator::peekable`. */
export function iterPeekable<T>(it: Iter<T>): Peekable<T> {
	return new Peekable(it);
}

/** `flat_map`: `into` makes each of `f`'s results an iterator, as `IntoIterator` does. */
export function iterFlatMap<T, V>(it: Iter<T>, f: (item: T) => Iter<V>): Iter<V>;
export function iterFlatMap<T, U, V>(it: Iter<T>, f: (item: T) => U, into: (value: U) => Iter<V>): Iter<V>;
export function iterFlatMap<T, U, V>(it: Iter<T>, f: (item: T) => U, into?: (value: U) => Iter<V>): Iter<V> {
	return new FlattenIter(new MapIter(it, f), into ?? (itself as (value: U) => Iter<V>));
}

/** `flatten`: `into` makes each item an iterator, as `IntoIterator` does. */
export function iterFlatten<V>(it: Iter<Iter<V>>): Iter<V>;
export function iterFlatten<T, V>(it: Iter<T>, into: (value: T) => Iter<V>): Iter<V>;
export function iterFlatten<T, V>(it: Iter<T>, into?: (value: T) => Iter<V>): Iter<V> {
	return new FlattenIter(it, into ?? (itself as (value: T) => Iter<V>));
}

// Consumers

/** `Iterator::count`. */
export function iterCount<T>(it: Iter<T>): number {
	let n = 0;
	while (it.next() !== DONE) n++;
	return n;
}

/** `Iterator::last`. */
export function iterLast<T>(it: Iter<T>): T | undefined {
	let last: T | undefined;
	for (let item = it.next(); item !== DONE; item = it.next()) last = item;
	return last;
}

/** `Iterator::nth`. */
export function iterNth<T>(it: Iter<T>, n: number): T | undefined {
	return iterToOption(it.nthItem(n));
}

/** `Iterator::any`. */
export function iterAny<T>(it: Iter<T>, f: (item: T) => boolean): boolean {
	for (let item = it.next(); item !== DONE; item = it.next()) if (f(item)) return true;
	return false;
}

/** `Iterator::all`. */
export function iterAll<T>(it: Iter<T>, f: (item: T) => boolean): boolean {
	for (let item = it.next(); item !== DONE; item = it.next()) if (!f(item)) return false;
	return true;
}

/** `find`, as `next` returns it. */
function findItem<T>(it: Iter<T>, f: (item: T) => boolean): T | Done {
	for (let item = it.next(); item !== DONE; item = it.next()) if (f(item)) return item;
	return DONE;
}

/** `rfind`, as `next_back` returns it. */
function rfindItem<T>(it: Iter<T>, f: (item: T) => boolean): T | Done {
	for (let item = it.nextBack(); item !== DONE; item = it.nextBack()) if (f(item)) return item;
	return DONE;
}

/** `Iterator::find`. */
export function iterFind<T>(it: Iter<T>, f: (item: T) => boolean): T | undefined {
	return iterToOption(findItem(it, f));
}

/** `Iterator::find_map`. */
export function iterFindMap<T, U>(it: Iter<T>, f: (item: T) => U | undefined): U | undefined {
	for (let item = it.next(); item !== DONE; item = it.next()) {
		const found = f(item);
		if (found !== undefined) return found;
	}
	return undefined;
}

/** `Iterator::position`. */
export function iterPosition<T>(it: Iter<T>, f: (item: T) => boolean): number | undefined {
	let i = 0;
	for (let item = it.next(); item !== DONE; item = it.next()) {
		if (f(item)) return i;
		i++;
	}
	return undefined;
}

/** `Iterator::fold`. */
export function iterFold<T, A>(it: Iter<T>, init: A, f: (acc: A, item: T) => A): A {
	let acc = init;
	for (let item = it.next(); item !== DONE; item = it.next()) acc = f(acc, item);
	return acc;
}

/** `Iterator::for_each`. */
export function iterForEach<T>(it: Iter<T>, f: (item: T) => void): void {
	for (let item = it.next(); item !== DONE; item = it.next()) f(item);
}

/** `sum`, adding with an integer type's checked `add`, so overflow panics as Rust's does. */
export function iterSum<T>(it: Iter<T>, add: (a: T, b: T) => T, zero: T): T {
	return iterFold(it, zero, add);
}

/** `product`, multiplying with an integer type's checked `mul`. */
export function iterProduct<T>(it: Iter<T>, mul: (a: T, b: T) => T, one: T): T {
	return iterFold(it, one, mul);
}

/** `max` of integers or booleans: the last of the greatest, as `Ord::max` keeps it. */
export function iterMax<K extends number | Limbs | boolean>(it: Iter<K>): K | undefined {
	return iterMaxByKey(it, (item) => item);
}

/** `min` of integers or booleans: the first of the least. */
export function iterMin<K extends number | Limbs | boolean>(it: Iter<K>): K | undefined {
	return iterMinByKey(it, (item) => item);
}

/** `max_by_key` with an integer or boolean key, each item's taken once. */
export function iterMaxByKey<T, K extends number | Limbs | boolean>(it: Iter<T>, f: (item: T) => K): T | undefined {
	let best = it.next();
	if (best === DONE) return undefined;
	let key = f(best);
	for (let item = it.next(); item !== DONE; item = it.next()) {
		const k = f(item);
		if (!lt(k, key)) [best, key] = [item, k];
	}
	return best;
}

/** `min_by_key` with an integer or boolean key, each item's taken once. */
export function iterMinByKey<T, K extends number | Limbs | boolean>(it: Iter<T>, f: (item: T) => K): T | undefined {
	let best = it.next();
	if (best === DONE) return undefined;
	let key = f(best);
	for (let item = it.next(); item !== DONE; item = it.next()) {
		const k = f(item);
		if (lt(k, key)) [best, key] = [item, k];
	}
	return best;
}

/** `collect` into a `Vec`. */
export function iterToArray<T>(it: Iter<T>): T[] {
	const out: T[] = [];
	for (let item = it.next(); item !== DONE; item = it.next()) out.push(item);
	return out;
}

/** `collect` into a `Vec<u8>`. */
export function iterToByteVec(it: Iter<number>): ByteVec {
	const out = new ByteVec();
	// Its own loop: through `iterExtendBytes`, JavaScriptCore took 1.9x as long.
	for (let item = it.next(); item !== DONE; item = it.next()) byteVecPush(out, item);
	return out;
}

/** `collect` of `Result`s: the first error, or every value collected by `into`. */
export function iterCollectResult<U, E, C>(it: Iter<Result<U, E>>, into: (values: Iter<U>) => C): Result<C, E> {
	const shunt = new ResultShunt(it);
	const values = into(shunt);
	return shunt.error === undefined ? Ok(values) : Err(shunt.error.value);
}

/** `collect` of `Option`s: `None` at the first `None`, or every value collected by `into`. */
export function iterCollectOption<U, C>(it: Iter<U | undefined>, into: (values: Iter<U>) => C): C | undefined {
	const shunt = new OptionShunt(it);
	const values = into(shunt);
	return shunt.none ? undefined : values;
}

class SliceIter<T> extends Iter<T> {
	private readonly s: ArrayLike<T>;
	private front = 0;
	private back: number;

	constructor(s: ArrayLike<T>) {
		super();
		this.s = s;
		this.back = s.length;
	}

	next(): T | Done {
		return this.front < this.back ? this.s[this.front++] : DONE;
	}

	override nextBack(): T | Done {
		return this.front < this.back ? this.s[--this.back] : DONE;
	}

	override len(): number {
		return this.back - this.front;
	}
}

/** `option::IntoIter`, `Once`, and `Empty`: at most one item. */
class OptionIter<T> extends Iter<T> {
	private item: T | undefined;
	private some: boolean;

	constructor(item: T | undefined, some = item !== undefined) {
		super();
		this.item = item;
		this.some = some;
	}

	next(): T | Done {
		if (!this.some) return DONE;
		this.some = false;
		const item = this.item as T;
		this.item = undefined;
		return item;
	}

	override nextBack(): T | Done {
		return this.next();
	}

	override len(): number {
		return this.some ? 1 : 0;
	}
}

/** An iterator whose `next` is a function of its source. */
class FromNext<S, T> extends Iter<T> {
	private readonly source: S;
	private readonly step: (source: S) => T | Done;

	constructor(source: S, step: (source: S) => T | Done) {
		super();
		this.source = source;
		this.step = step;
	}

	next(): T | Done {
		return this.step(this.source);
	}
}

class MapIter<T, U> extends Iter<U> {
	private readonly inner: Iter<T>;
	private readonly f: (item: T) => U;

	constructor(inner: Iter<T>, f: (item: T) => U) {
		super();
		this.inner = inner;
		this.f = f;
	}

	next(): U | Done {
		const item = this.inner.next();
		return item === DONE ? DONE : this.f(item);
	}

	override nextBack(): U | Done {
		const item = this.inner.nextBack();
		return item === DONE ? DONE : this.f(item);
	}

	override len(): number {
		return this.inner.len();
	}
}

class FilterIter<T> extends Iter<T> {
	private readonly inner: Iter<T>;
	private readonly f: (item: T) => boolean;

	constructor(inner: Iter<T>, f: (item: T) => boolean) {
		super();
		this.inner = inner;
		this.f = f;
	}

	next(): T | Done {
		return findItem(this.inner, this.f);
	}

	override nextBack(): T | Done {
		return rfindItem(this.inner, this.f);
	}
}

class FilterMapIter<T, U> extends Iter<U> {
	private readonly inner: Iter<T>;
	private readonly f: (item: T) => U | undefined;

	constructor(inner: Iter<T>, f: (item: T) => U | undefined) {
		super();
		this.inner = inner;
		this.f = f;
	}

	next(): U | Done {
		for (let item = this.inner.next(); item !== DONE; item = this.inner.next()) {
			const mapped = this.f(item);
			if (mapped !== undefined) return mapped;
		}
		return DONE;
	}

	override nextBack(): U | Done {
		for (let item = this.inner.nextBack(); item !== DONE; item = this.inner.nextBack()) {
			const mapped = this.f(item);
			if (mapped !== undefined) return mapped;
		}
		return DONE;
	}
}

class EnumerateIter<T> extends Iter<[number, T]> {
	private readonly inner: Iter<T>;
	private index = 0;

	constructor(inner: Iter<T>) {
		super();
		this.inner = inner;
	}

	next(): [number, T] | Done {
		const item = this.inner.next();
		return item === DONE ? DONE : [this.index++, item];
	}

	override nextBack(): [number, T] | Done {
		const item = this.inner.nextBack();
		return item === DONE ? DONE : [this.index + this.inner.len(), item];
	}

	/** std's `nth`, which counts only when it finds an item. */
	override nthItem(n: number): [number, T] | Done {
		const item = this.inner.nthItem(n);
		if (item === DONE) return DONE;
		const i = this.index + n;
		this.index = i + 1;
		return [i, item];
	}

	override nthBackItem(n: number): [number, T] | Done {
		const item = this.inner.nthBackItem(n);
		return item === DONE ? DONE : [this.index + this.inner.len(), item];
	}

	override len(): number {
		return this.inner.len();
	}
}

/** `Chain`, which stops reading its first iterator once it ends, but not its second. */
class ChainIter<T> extends Iter<T> {
	private a: Iter<T> | undefined;
	private b: Iter<T> | undefined;

	constructor(a: Iter<T>, b: Iter<T>) {
		super();
		this.a = a;
		this.b = b;
	}

	next(): T | Done {
		if (this.a !== undefined) {
			const item = this.a.next();
			if (item !== DONE) return item;
			this.a = undefined;
		}
		return this.b === undefined ? DONE : this.b.next();
	}

	override nextBack(): T | Done {
		if (this.b !== undefined) {
			const item = this.b.nextBack();
			if (item !== DONE) return item;
			this.b = undefined;
		}
		return this.a === undefined ? DONE : this.a.nextBack();
	}
}

class ZipIter<A, B> extends Iter<[A, B]> {
	private readonly a: Iter<A>;
	private readonly b: Iter<B>;

	constructor(a: Iter<A>, b: Iter<B>) {
		super();
		this.a = a;
		this.b = b;
	}

	next(): [A, B] | Done {
		const a = this.a.next();
		if (a === DONE) return DONE;
		const b = this.b.next();
		return b === DONE ? DONE : [a, b];
	}

	/** std's: the longer side drops its extra items from the back first. */
	override nextBack(): [A, B] | Done {
		const aLen = this.a.len();
		const bLen = this.b.len();
		for (let i = bLen; i < aLen; i++) this.a.nextBack();
		for (let i = aLen; i < bLen; i++) this.b.nextBack();
		const a = this.a.nextBack();
		const b = this.b.nextBack();
		if (a === DONE && b === DONE) return DONE;
		if (a === DONE || b === DONE) panic("internal error: entered unreachable code");
		return [a, b];
	}

	override len(): number {
		return Math.min(this.a.len(), this.b.len());
	}
}

/** std's `Zip` of two `TrustedRandomAccess` iterators, which counts the pairs it reads. */
class TrustedZipIter<A, B> extends ZipIter<A, B> {
	private index = 0;
	private end: number;

	constructor(a: Iter<A>, b: Iter<B>) {
		super(a, b);
		this.end = Math.min(a.len(), b.len());
	}

	override next(): [A, B] | Done {
		if (this.index >= this.end) return DONE;
		this.index++;
		return super.next();
	}

	override nextBack(): [A, B] | Done {
		if (this.index >= this.end) return DONE;
		this.end--;
		return super.nextBack();
	}

	override len(): number {
		return this.end - this.index;
	}
}

class RevIter<T> extends Iter<T> {
	private readonly inner: Iter<T>;

	constructor(inner: Iter<T>) {
		super();
		this.inner = inner;
	}

	next(): T | Done {
		return this.inner.nextBack();
	}

	override nextBack(): T | Done {
		return this.inner.next();
	}

	override nthItem(n: number): T | Done {
		return this.inner.nthBackItem(n);
	}

	override nthBackItem(n: number): T | Done {
		return this.inner.nthItem(n);
	}

	override len(): number {
		return this.inner.len();
	}
}

class SkipIter<T> extends Iter<T> {
	private readonly inner: Iter<T>;
	private n: number;

	constructor(inner: Iter<T>, n: number) {
		super();
		this.inner = inner;
		this.n = n;
	}

	next(): T | Done {
		if (this.n === 0) return this.inner.next();
		const n = this.n;
		this.n = 0;
		return this.inner.nthItem(n);
	}

	override nextBack(): T | Done {
		return this.len() > 0 ? this.inner.nextBack() : DONE;
	}

	override len(): number {
		return Math.max(this.inner.len() - this.n, 0);
	}
}

class TakeIter<T> extends Iter<T> {
	private readonly inner: Iter<T>;
	private n: number;

	constructor(inner: Iter<T>, n: number) {
		super();
		this.inner = inner;
		this.n = n;
	}

	next(): T | Done {
		if (this.n === 0) return DONE;
		this.n--;
		return this.inner.next();
	}

	override nextBack(): T | Done {
		if (this.n === 0) return DONE;
		const n = this.n--;
		return this.inner.nthBackItem(Math.max(this.inner.len() - n, 0));
	}

	/** std's `nth`: past the limit, it reads the rest of the limit through and ends. */
	override nthItem(n: number): T | Done {
		if (this.n > n) {
			this.n -= n + 1;
			return this.inner.nthItem(n);
		}
		if (this.n > 0) {
			this.inner.nthItem(this.n - 1);
			this.n = 0;
		}
		return DONE;
	}

	/** std's `nth_back`: past the limit, it reads the whole rest from the back, even at a limit of 0. */
	override nthBackItem(n: number): T | Done {
		const len = this.inner.len();
		if (this.n > n) {
			const m = Math.max(len - this.n, 0) + n;
			this.n -= n + 1;
			return this.inner.nthBackItem(m);
		}
		if (len > 0) this.inner.nthBackItem(len - 1);
		return DONE;
	}

	override len(): number {
		return Math.min(this.inner.len(), this.n);
	}
}

class SkipWhileIter<T> extends Iter<T> {
	private readonly inner: Iter<T>;
	private readonly f: (item: T) => boolean;
	private flag = false;

	constructor(inner: Iter<T>, f: (item: T) => boolean) {
		super();
		this.inner = inner;
		this.f = f;
	}

	next(): T | Done {
		return findItem(this.inner, (item) => {
			if (this.flag || !this.f(item)) {
				this.flag = true;
				return true;
			}
			return false;
		});
	}
}

class TakeWhileIter<T> extends Iter<T> {
	private readonly inner: Iter<T>;
	private readonly f: (item: T) => boolean;
	private flag = false;

	constructor(inner: Iter<T>, f: (item: T) => boolean) {
		super();
		this.inner = inner;
		this.f = f;
	}

	next(): T | Done {
		if (this.flag) return DONE;
		const item = this.inner.next();
		if (item === DONE) return DONE;
		if (this.f(item)) return item;
		this.flag = true;
		return DONE;
	}
}

/** `MapWhile`, which reads on after `f` returns `None` if called again, as std's does. */
class MapWhileIter<T, U> extends Iter<U> {
	private readonly inner: Iter<T>;
	private readonly f: (item: T) => U | undefined;

	constructor(inner: Iter<T>, f: (item: T) => U | undefined) {
		super();
		this.inner = inner;
		this.f = f;
	}

	next(): U | Done {
		const item = this.inner.next();
		return item === DONE ? DONE : iterFromOption(this.f(item));
	}
}

class StepByIter<T> extends Iter<T> {
	private readonly inner: Iter<T>;
	private readonly step: number;
	private firstTake = true;

	constructor(inner: Iter<T>, step: number) {
		super();
		if (step === 0) panic("assertion failed: step != 0");
		this.inner = inner;
		this.step = step - 1;
	}

	next(): T | Done {
		const n = this.firstTake ? 0 : this.step;
		this.firstTake = false;
		return this.inner.nthItem(n);
	}

	override nextBack(): T | Done {
		const rem = this.inner.len() % (this.step + 1);
		const n = this.firstTake ? (rem === 0 ? this.step : rem - 1) : rem;
		return this.inner.nthBackItem(n);
	}

	override len(): number {
		const n = this.inner.len();
		if (this.firstTake) return n === 0 ? 0 : 1 + Math.floor((n - 1) / (this.step + 1));
		return Math.floor(n / (this.step + 1));
	}
}

class InspectIter<T> extends Iter<T> {
	private readonly inner: Iter<T>;
	private readonly f: (item: T) => void;

	constructor(inner: Iter<T>, f: (item: T) => void) {
		super();
		this.inner = inner;
		this.f = f;
	}

	next(): T | Done {
		const item = this.inner.next();
		if (item !== DONE) this.f(item);
		return item;
	}

	override nextBack(): T | Done {
		const item = this.inner.nextBack();
		if (item !== DONE) this.f(item);
		return item;
	}

	override len(): number {
		return this.inner.len();
	}
}

class FuseIter<T> extends Iter<T> {
	private inner: Iter<T> | undefined;

	constructor(inner: Iter<T>) {
		super();
		this.inner = inner;
	}

	next(): T | Done {
		if (this.inner === undefined) return DONE;
		const item = this.inner.next();
		if (item === DONE) this.inner = undefined;
		return item;
	}

	override nextBack(): T | Done {
		if (this.inner === undefined) return DONE;
		const item = this.inner.nextBack();
		if (item === DONE) this.inner = undefined;
		return item;
	}

	override len(): number {
		return this.inner === undefined ? 0 : this.inner.len();
	}
}

/** `Flatten` and `FlatMap`, which stop reading their outer iterator once it ends. */
class FlattenIter<T, V> extends Iter<V> {
	private outer: Iter<T> | undefined;
	private readonly into: (value: T) => Iter<V>;
	private front: Iter<V> | undefined;
	private back: Iter<V> | undefined;

	constructor(outer: Iter<T>, into: (value: T) => Iter<V>) {
		super();
		this.outer = outer;
		this.into = into;
	}

	next(): V | Done {
		for (;;) {
			if (this.front !== undefined) {
				const item = this.front.next();
				if (item !== DONE) return item;
				this.front = undefined;
			}
			const inner = this.outer === undefined ? DONE : this.outer.next();
			if (inner === DONE) {
				this.outer = undefined;
				if (this.back === undefined) return DONE;
				const item = this.back.next();
				if (item === DONE) this.back = undefined;
				return item;
			}
			this.front = this.into(inner);
		}
	}

	override nextBack(): V | Done {
		for (;;) {
			if (this.back !== undefined) {
				const item = this.back.nextBack();
				if (item !== DONE) return item;
				this.back = undefined;
			}
			const inner = this.outer === undefined ? DONE : this.outer.nextBack();
			if (inner === DONE) {
				this.outer = undefined;
				if (this.front === undefined) return DONE;
				const item = this.front.nextBack();
				if (item === DONE) this.front = undefined;
				return item;
			}
			this.back = this.into(inner);
		}
	}
}

/** `Peekable`. */
export class Peekable<T> extends Iter<T> {
	readonly inner: Iter<T>;
	peeked = false;
	item: T | Done = DONE;

	constructor(inner: Iter<T>) {
		super();
		this.inner = inner;
	}

	next(): T | Done {
		if (!this.peeked) return this.inner.next();
		this.peeked = false;
		return this.item;
	}

	override nextBack(): T | Done {
		if (!this.peeked) return this.inner.nextBack();
		if (this.item === DONE) return DONE;
		const item = this.inner.nextBack();
		if (item !== DONE) return item;
		// The peeked item is the last: taking it leaves the end peeked.
		const peeked = this.item;
		this.item = DONE;
		return peeked;
	}

	override len(): number {
		if (!this.peeked) return this.inner.len();
		return this.item === DONE ? 0 : this.inner.len() + 1;
	}
}

/** `Peekable::peek`. */
export function peekablePeek<T>(p: Peekable<T>): T | undefined {
	if (!p.peeked) {
		p.item = p.inner.next();
		p.peeked = true;
	}
	return iterToOption(p.item);
}

/** `Peekable::next_if`. */
export function peekableNextIf<T>(p: Peekable<T>, f: (item: T) => boolean): T | undefined {
	const item = p.next();
	if (item !== DONE && f(item)) return item;
	p.peeked = true;
	p.item = item;
	return undefined;
}

/** What `iterCollectResult` collects from: values until the first error, which it keeps. */
class ResultShunt<U, E> extends Iter<U> {
	private readonly inner: Iter<Result<U, E>>;
	error: { value: E } | undefined;

	constructor(inner: Iter<Result<U, E>>) {
		super();
		this.inner = inner;
	}

	next(): U | Done {
		if (this.error !== undefined) return DONE;
		const item = this.inner.next();
		if (item === DONE) return DONE;
		if (item.ok) return item.value;
		this.error = { value: item.error };
		return DONE;
	}
}

/** What `iterCollectOption` collects from: values until the first `None`. */
class OptionShunt<U> extends Iter<U> {
	private readonly inner: Iter<U | undefined>;
	none = false;

	constructor(inner: Iter<U | undefined>) {
		super();
		this.inner = inner;
	}

	next(): U | Done {
		if (this.none) return DONE;
		const item = this.inner.next();
		if (item === DONE) return DONE;
		if (item !== undefined) return item;
		this.none = true;
		return DONE;
	}
}

/** `a < b` of two integers of one type, or two booleans. */
function lt<T>(a: T, b: T): boolean {
	if (typeof a === "object" || typeof b === "object") return wideLt(a as number | Limbs, b as number | Limbs);
	return (a as number) < (b as number);
}

/**
 * `n + by` in the integer type `int` steps, or `undefined` past its range, which a range
 * stepping past its end only needs to stay past. A narrow integer's sum is an exact
 * number whatever its type.
 */
function step<T>(n: T, by: number, int: Step<T> | undefined): T | undefined {
	if (typeof n === "number" && Number.isSafeInteger(n + by)) return (n + by) as T;
	if (int === undefined) throw new Error("rust2-ts: stepping a wide range without its type");
	const [sum, over] = int(n, by);
	return over ? undefined : sum;
}

/**
 * `ops::Range`: `start..end`, which iterates its integers by moving `start`. A 64- or
 * 128-bit range takes its type's `Step`, `rt.u64Step`, to step past 2^53.
 */
export class Range<T> extends Iter<T> {
	start: T;
	end: T;
	readonly int: Step<T> | undefined;

	constructor(start: T, end: T, int?: Step<T>) {
		super();
		this.start = start;
		this.end = end;
		this.int = int;
	}

	next(): T | Done {
		const start = this.start;
		if (!lt(start, this.end)) return DONE;
		this.start = step(start, 1, this.int)!;
		return start;
	}

	override nextBack(): T | Done {
		if (!lt(this.start, this.end)) return DONE;
		this.end = step(this.end, -1, this.int)!;
		return this.end;
	}

	/** std's `nth`, in one step: past the end, the range ends. */
	override nthItem(n: number): T | Done {
		const item = step(this.start, n, this.int);
		if (item !== undefined && lt(item, this.end)) {
			this.start = step(item, 1, this.int)!;
			return item;
		}
		this.start = this.end;
		return DONE;
	}

	/** std's `nth_back`, in one step. */
	override nthBackItem(n: number): T | Done {
		const item = step(this.end, -n, this.int);
		if (item !== undefined && lt(this.start, item)) {
			this.end = step(item, -1, this.int)!;
			return this.end;
		}
		this.end = this.start;
		return DONE;
	}

	override len(): number {
		return lt(this.start, this.end) ? Number(this.end) - Number(this.start) : 0;
	}
}

/** `Range::is_empty`. */
export function rangeIsEmpty<T>(r: Range<T>): boolean {
	return !lt(r.start, r.end);
}

/** `Range::contains`. */
export function rangeContains<T>(r: Range<T>, item: T): boolean {
	return !lt(item, r.start) && lt(item, r.end);
}

/** `Range::clone`. */
export function rangeClone<T>(r: Range<T>): Range<T> {
	return new Range(r.start, r.end, r.int);
}

/** `ops::RangeInclusive`: `start..=end`, which remembers once it has yielded `end`. */
export class RangeInclusive<T> extends Iter<T> {
	lo: T;
	hi: T;
	exhausted = false;
	/** The integer type, which `nth` wraps within as std's does. */
	readonly int: Step<T> | undefined;

	constructor(start: T, end: T, int?: Step<T>) {
		super();
		this.lo = start;
		this.hi = end;
		this.int = int;
	}

	/** std's: `start` steps past the item, so it passes `end`, and only wrapping exhausts it. */
	next(): T | Done {
		if (rangeInclusiveIsEmpty(this)) return DONE;
		const lo = this.lo;
		[this.lo, this.exhausted] = stepOverflowing(lo, 1, this.int);
		return lo;
	}

	override nextBack(): T | Done {
		if (rangeInclusiveIsEmpty(this)) return DONE;
		const hi = this.hi;
		[this.hi, this.exhausted] = stepOverflowing(hi, -1, this.int);
		return hi;
	}

	override len(): number {
		return rangeInclusiveIsEmpty(this) ? 0 : Number(this.hi) - Number(this.lo) + 1;
	}

	/** std's `nth`: `start` moves past the item, wrapping if it overflows, which ends the range. */
	override nthItem(n: number): T | Done {
		if (rangeInclusiveIsEmpty(this)) return DONE;
		const [item, over] = overflowing(this.lo, n, this.int);
		const [after, overAfter] = overflowing(item, 1, this.int);
		this.lo = after;
		this.exhausted = over || overAfter;
		return !over && !lt(this.hi, item) ? item : DONE;
	}

	/** std's `nth_back`, from the end. */
	override nthBackItem(n: number): T | Done {
		if (rangeInclusiveIsEmpty(this)) return DONE;
		const [item, over] = overflowing(this.hi, -n, this.int);
		const [before, overBefore] = overflowing(item, -1, this.int);
		this.hi = before;
		this.exhausted = over || overBefore;
		return !over && !lt(item, this.lo) ? item : DONE;
	}
}

/** `RangeInclusive::start`. */
export function rangeInclusiveStart<T>(r: RangeInclusive<T>): T {
	return r.lo;
}

/** `RangeInclusive::end`. */
export function rangeInclusiveEnd<T>(r: RangeInclusive<T>): T {
	return r.hi;
}

/** `RangeInclusive::is_empty`. */
export function rangeInclusiveIsEmpty<T>(r: RangeInclusive<T>): boolean {
	return r.exhausted || lt(r.hi, r.lo);
}

/** `RangeInclusive::contains`. */
export function rangeInclusiveContains<T>(r: RangeInclusive<T>, item: T): boolean {
	// Exhausted, it may have wrapped past its end, and contains nothing, as std's bounds say.
	return !r.exhausted && !lt(item, r.lo) && !lt(r.hi, item);
}

/** `RangeInclusive::clone`. */
export function rangeInclusiveClone<T>(r: RangeInclusive<T>): RangeInclusive<T> {
	const range = new RangeInclusive(r.lo, r.hi, r.int);
	range.exhausted = r.exhausted;
	return range;
}

/** `RangeInclusive::start_bound`: its start, included. */
export function rangeInclusiveStartBound<T>(r: RangeInclusive<T>): Bound<T> {
	return { tag: "Included", _0: r.lo };
}

/** `RangeInclusive::end_bound`: its end, included, until wrapping exhausts it and it excludes its start, as std's. */
export function rangeInclusiveEndBound<T>(r: RangeInclusive<T>): Bound<T> {
	return r.exhausted ? { tag: "Excluded", _0: r.lo } : { tag: "Included", _0: r.hi };
}

/** `Step::forward_overflowing` and `backward_overflowing`: `n + by`, wrapped within `int`, and whether it wrapped. */
/** `Step::forward_overflowing` by `by`, of an integer type's step or a safe number's. */
function stepOverflowing<T>(n: T, by: number, int: Step<T> | undefined): [T, boolean] {
	if (int !== undefined) return int(n, by);
	return [step(n, by, int)!, false];
}

function overflowing<T>(n: T, by: number, int: Step<T> | undefined): [T, boolean] {
	if (int === undefined) throw new Error("rust2-ts: stepping a range of no integer type");
	return int(n, by);
}

/** An iterator as itself: what `into_iter` makes of one. */
function itself<T>(iter: Iter<T>): Iter<T> {
	return iter;
}
