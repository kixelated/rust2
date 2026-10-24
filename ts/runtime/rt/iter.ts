// Iterators, for rust2-ts output: std's `Iterator`, its adaptors, and `Range`, lazy as
// Rust's are. `rt.ts` re-exports all of it.
//
// `next` returns an item or `DONE`, never `undefined`, so an iterator of `Option<T>` keeps
// its `None` items apart from its end. Each adaptor calls what it wraps, and the closures
// it was given, in the order and as many times as std's does, and stops where std's
// stops: a `find` or `any` reads no further than its match.

import { ByteVec, Err, Ok, panic, type Result } from "../rt.ts";

/** What `next` returns once an iterator is exhausted, which no item is. */
export const DONE: unique symbol = Symbol("DONE");

/** The type of `DONE`. */
export type Done = typeof DONE;

/** `Iterator`, with the provided methods std's iterators share. */
export abstract class Iter<T> implements Iterable<T> {
	/** The next item, or `DONE`. */
	abstract next(): T | Done;

	/** `DoubleEndedIterator::next_back`, which Rust calls only where std implements it. */
	nextBack(): T | Done {
		throw new Error("rust2-ts: an iterator without `next_back`");
	}

	/** `ExactSizeIterator::len`, which Rust calls only where std implements it. */
	len(): number {
		throw new Error("rust2-ts: an iterator without `len`");
	}

	/** Its items, for a JS `for ... of`. */
	*[Symbol.iterator](): Iterator<T> {
		for (let item = this.next(); item !== DONE; item = this.next()) yield item;
	}

	// Adaptors

	map<U>(f: (item: T) => U): Iter<U> {
		return new MapIter(this, f);
	}

	filter(f: (item: T) => boolean): Iter<T> {
		return new FilterIter(this, f);
	}

	filterMap<U>(f: (item: T) => U | undefined): Iter<U> {
		return new FilterMapIter(this, f);
	}

	enumerate(): Iter<[number, T]> {
		return new EnumerateIter(this);
	}

	/** `chain`, with `other` already made an iterator, as std does when it is called. */
	chain(other: Iter<T>): Iter<T> {
		return new ChainIter(this, other);
	}

	/** `zip`, with `other` already made an iterator, as std does when it is called. */
	zip<U>(other: Iter<U>): Iter<[T, U]> {
		return new ZipIter(this, other);
	}

	rev(): Iter<T> {
		return new RevIter(this);
	}

	skip(n: number): Iter<T> {
		return new SkipIter(this, n);
	}

	take(n: number): Iter<T> {
		return new TakeIter(this, n);
	}

	skipWhile(f: (item: T) => boolean): Iter<T> {
		return new SkipWhileIter(this, f);
	}

	takeWhile(f: (item: T) => boolean): Iter<T> {
		return new TakeWhileIter(this, f);
	}

	mapWhile<U>(f: (item: T) => U | undefined): Iter<U> {
		return new MapWhileIter(this, f);
	}

	stepBy(step: number): Iter<T> {
		return new StepByIter(this, step);
	}

	inspect(f: (item: T) => void): Iter<T> {
		return new InspectIter(this, f);
	}

	fuse(): Iter<T> {
		return new FuseIter(this);
	}

	peekable(): Peekable<T> {
		return new Peekable(this);
	}

	/** `flat_map`: `into` makes each of `f`'s results an iterator, as `IntoIterator` does. */
	flatMap<V>(f: (item: T) => Iter<V>): Iter<V>;
	flatMap<U, V>(f: (item: T) => U, into: (value: U) => Iter<V>): Iter<V>;
	flatMap<U, V>(f: (item: T) => U, into?: (value: U) => Iter<V>): Iter<V> {
		return new FlattenIter(new MapIter(this, f), into ?? (itself as (value: U) => Iter<V>));
	}

	/** `flatten`: `into` makes each item an iterator, as `IntoIterator` does. */
	flatten<V>(this: Iter<Iter<V>>): Iter<V>;
	flatten<V>(into: (value: T) => Iter<V>): Iter<V>;
	flatten<V>(into?: (value: T) => Iter<V>): Iter<V> {
		return new FlattenIter(this, into ?? (itself as (value: T) => Iter<V>));
	}

	// Consumers

	count(): number {
		let n = 0;
		while (this.next() !== DONE) n++;
		return n;
	}

	last(): T | undefined {
		let last: T | undefined;
		for (let item = this.next(); item !== DONE; item = this.next()) last = item;
		return last;
	}

	/** `advance_by`: whether `n` items were there to skip. */
	advanceBy(n: number): boolean {
		for (let i = 0; i < n; i++) if (this.next() === DONE) return false;
		return true;
	}

	/** `advance_back_by`. */
	advanceBackBy(n: number): boolean {
		for (let i = 0; i < n; i++) if (this.nextBack() === DONE) return false;
		return true;
	}

	/** `nth`, as `next` returns it. */
	nthItem(n: number): T | Done {
		return this.advanceBy(n) ? this.next() : DONE;
	}

	/** `nth_back`, as `next_back` returns it. */
	nthBackItem(n: number): T | Done {
		return this.advanceBackBy(n) ? this.nextBack() : DONE;
	}

	nth(n: number): T | undefined {
		return toOption(this.nthItem(n));
	}

	any(f: (item: T) => boolean): boolean {
		for (let item = this.next(); item !== DONE; item = this.next()) if (f(item)) return true;
		return false;
	}

	all(f: (item: T) => boolean): boolean {
		for (let item = this.next(); item !== DONE; item = this.next()) if (!f(item)) return false;
		return true;
	}

	/** `find`, as `next` returns it. */
	findItem(f: (item: T) => boolean): T | Done {
		for (let item = this.next(); item !== DONE; item = this.next()) if (f(item)) return item;
		return DONE;
	}

	/** `rfind`, as `next_back` returns it. */
	rfindItem(f: (item: T) => boolean): T | Done {
		for (let item = this.nextBack(); item !== DONE; item = this.nextBack()) if (f(item)) return item;
		return DONE;
	}

	find(f: (item: T) => boolean): T | undefined {
		return toOption(this.findItem(f));
	}

	findMap<U>(f: (item: T) => U | undefined): U | undefined {
		for (let item = this.next(); item !== DONE; item = this.next()) {
			const found = f(item);
			if (found !== undefined) return found;
		}
		return undefined;
	}

	position(f: (item: T) => boolean): number | undefined {
		let i = 0;
		for (let item = this.next(); item !== DONE; item = this.next()) {
			if (f(item)) return i;
			i++;
		}
		return undefined;
	}

	fold<A>(init: A, f: (acc: A, item: T) => A): A {
		let acc = init;
		for (let item = this.next(); item !== DONE; item = this.next()) acc = f(acc, item);
		return acc;
	}

	forEach(f: (item: T) => void): void {
		for (let item = this.next(); item !== DONE; item = this.next()) f(item);
	}

	/** `sum`, adding with an integer type's checked `add`, so overflow panics as Rust's does. */
	sum(add: (a: T, b: T) => T, zero: T): T {
		return this.fold(zero, add);
	}

	/** `product`, multiplying with an integer type's checked `mul`. */
	product(mul: (a: T, b: T) => T, one: T): T {
		return this.fold(one, mul);
	}

	/** `max` of integers or booleans: the last of the greatest, as `Ord::max` keeps it. */
	max<K extends number | bigint | boolean>(this: Iter<K>): K | undefined {
		return this.maxByKey((item) => item);
	}

	/** `min` of integers or booleans: the first of the least. */
	min<K extends number | bigint | boolean>(this: Iter<K>): K | undefined {
		return this.minByKey((item) => item);
	}

	/** `max_by_key` with an integer or boolean key, each item's taken once. */
	maxByKey<K extends number | bigint | boolean>(f: (item: T) => K): T | undefined {
		let best = this.next();
		if (best === DONE) return undefined;
		let key = f(best);
		for (let item = this.next(); item !== DONE; item = this.next()) {
			const k = f(item);
			if (k >= key) [best, key] = [item, k];
		}
		return best;
	}

	/** `min_by_key` with an integer or boolean key, each item's taken once. */
	minByKey<K extends number | bigint | boolean>(f: (item: T) => K): T | undefined {
		let best = this.next();
		if (best === DONE) return undefined;
		let key = f(best);
		for (let item = this.next(); item !== DONE; item = this.next()) {
			const k = f(item);
			if (k < key) [best, key] = [item, k];
		}
		return best;
	}

	/** `collect` into a `Vec`. */
	toArray(): T[] {
		const out: T[] = [];
		for (let item = this.next(); item !== DONE; item = this.next()) out.push(item);
		return out;
	}

	/** `collect` into a `Vec<u8>`. */
	toByteVec(this: Iter<number>): ByteVec {
		const out = new ByteVec();
		for (let item = this.next(); item !== DONE; item = this.next()) out.push(item);
		return out;
	}

	/** `collect` of `Result`s: the first error, or every value collected by `into`. */
	collectResult<U, E, C>(this: Iter<Result<U, E>>, into: (values: Iter<U>) => C): Result<C, E> {
		const shunt = new ResultShunt(this);
		const values = into(shunt);
		return shunt.error === undefined ? Ok(values) : Err(shunt.error.value);
	}

	/** `collect` of `Option`s: `None` at the first `None`, or every value collected by `into`. */
	collectOption<U, C>(this: Iter<U | undefined>, into: (values: Iter<U>) => C): C | undefined {
		const shunt = new OptionShunt(this);
		const values = into(shunt);
		return shunt.none ? undefined : values;
	}
}

/** The item `next` returned, as an `Option`. */
function toOption<T>(item: T | Done): T | undefined {
	return item === DONE ? undefined : item;
}

/** An `Option`, as `next` returns it. */
function fromOption<T>(option: T | undefined): T | Done {
	return option === undefined ? DONE : option;
}

/** Where std's iterators come from. */
export const iter = {
	toOption,
	fromOption,
	/** `slice::Iter`, and what iterating a `Vec` or array by value makes. */
	slice<T>(s: ArrayLike<T>): Iter<T> {
		return new SliceIter(s);
	},
	/** `Option::iter`, and `option::IntoIter`. */
	option<T>(o: T | undefined): Iter<T> {
		return new OptionIter(o);
	},
	once<T>(value: T): Iter<T> {
		return new OptionIter<T>(value, true);
	},
	empty<T>(): Iter<T> {
		return new OptionIter<T>(undefined, false);
	},
	/** `iter::from_fn`, whose closure returns an `Option`. */
	fromFn<T>(f: () => T | undefined): Iter<T> {
		return new FromNext(f, (f) => fromOption(f()));
	},
	/** An `impl Iterator` of the translated crate, which `next` calls the `next` of. */
	from<S, T>(source: S, next: (source: S) => T | Done): Iter<T> {
		return new FromNext(source, next);
	},
	/** `Extend::extend` of a `Vec`. */
	extend<T>(v: { push(item: T): void }, items: Iter<T>): void {
		for (let item = items.next(); item !== DONE; item = items.next()) v.push(item);
	},
};

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
		return this.inner.findItem(this.f);
	}

	override nextBack(): T | Done {
		return this.inner.rfindItem(this.f);
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
		return this.inner.findItem((item) => {
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
		return item === DONE ? DONE : fromOption(this.f(item));
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
	private readonly inner: Iter<T>;
	private peeked = false;
	private item: T | Done = DONE;

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

	/** `peek`, as `next` returns it. */
	peekItem(): T | Done {
		if (!this.peeked) {
			this.item = this.inner.next();
			this.peeked = true;
		}
		return this.item;
	}

	peek(): T | undefined {
		return toOption(this.peekItem());
	}

	/** `next_if`, as `next` returns it. */
	nextIfItem(f: (item: T) => boolean): T | Done {
		const item = this.next();
		if (item !== DONE && f(item)) return item;
		this.peeked = true;
		this.item = item;
		return DONE;
	}

	nextIf(f: (item: T) => boolean): T | undefined {
		return toOption(this.nextIfItem(f));
	}
}

/** What `collectResult` collects from: values until the first error, which it keeps. */
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

/** What `collectOption` collects from: values until the first `None`. */
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

/** `a < b` of integers, numbers or bigints alike. */
function lt<T>(a: T, b: T): boolean {
	return (a as number) < (b as number);
}

/** `n + by`, in `n`'s representation. */
function step<T>(n: T, by: number): T {
	return (typeof n === "bigint" ? n + BigInt(by) : (n as number) + by) as T;
}

/** One past `n`. */
function succ<T>(n: T): T {
	return step(n, 1);
}

/** One before `n`. */
function pred<T>(n: T): T {
	return step(n, -1);
}

/** `ops::Range`: `start..end`, which iterates its integers by moving `start`. */
export class Range<T> extends Iter<T> {
	start: T;
	end: T;

	constructor(start: T, end: T) {
		super();
		this.start = start;
		this.end = end;
	}

	next(): T | Done {
		const start = this.start;
		if (!lt(start, this.end)) return DONE;
		this.start = succ(start);
		return start;
	}

	override nextBack(): T | Done {
		if (!lt(this.start, this.end)) return DONE;
		this.end = pred(this.end);
		return this.end;
	}

	/** std's `nth`, in one step: past the end, the range ends. */
	override nthItem(n: number): T | Done {
		const item = step(this.start, n);
		if (lt(item, this.end)) {
			this.start = succ(item);
			return item;
		}
		this.start = this.end;
		return DONE;
	}

	/** std's `nth_back`, in one step. */
	override nthBackItem(n: number): T | Done {
		const item = step(this.end, -n);
		if (lt(this.start, item)) {
			this.end = pred(item);
			return this.end;
		}
		this.end = this.start;
		return DONE;
	}

	override len(): number {
		return lt(this.start, this.end) ? Number(this.end) - Number(this.start) : 0;
	}

	isEmpty(): boolean {
		return !lt(this.start, this.end);
	}

	contains(item: T): boolean {
		return !lt(item, this.start) && lt(item, this.end);
	}

	clone(): Range<T> {
		return new Range(this.start, this.end);
	}
}

/** `ops::RangeInclusive`: `start..=end`, which remembers once it has yielded `end`. */
export class RangeInclusive<T> extends Iter<T> {
	private lo: T;
	private hi: T;
	private exhausted = false;
	/** The integer type's bounds, which `nth` wraps within as std's does. */
	private readonly bounds: Bounds<T> | undefined;

	constructor(start: T, end: T, bounds?: Bounds<T>) {
		super();
		this.lo = start;
		this.hi = end;
		this.bounds = bounds;
	}

	start(): T {
		return this.lo;
	}

	end(): T {
		return this.hi;
	}

	isEmpty(): boolean {
		return this.exhausted || lt(this.hi, this.lo);
	}

	contains(item: T): boolean {
		// Exhausted, it may have wrapped past its end, and contains nothing, as std's bounds say.
		return !this.exhausted && !lt(item, this.lo) && !lt(this.hi, item);
	}

	clone(): RangeInclusive<T> {
		const range = new RangeInclusive(this.lo, this.hi, this.bounds);
		range.exhausted = this.exhausted;
		return range;
	}

	next(): T | Done {
		if (this.isEmpty()) return DONE;
		const lo = this.lo;
		if (lt(lo, this.hi)) this.lo = succ(lo);
		else this.exhausted = true;
		return lo;
	}

	override nextBack(): T | Done {
		if (this.isEmpty()) return DONE;
		const hi = this.hi;
		if (lt(this.lo, hi)) this.hi = pred(hi);
		else this.exhausted = true;
		return hi;
	}

	override len(): number {
		return this.isEmpty() ? 0 : Number(this.hi) - Number(this.lo) + 1;
	}

	/** std's `nth`: `start` moves past the item, wrapping if it overflows, which ends the range. */
	override nthItem(n: number): T | Done {
		if (this.isEmpty()) return DONE;
		const [item, over] = overflowing(this.lo, n, this.bounds);
		const [after, overAfter] = overflowing(item, 1, this.bounds);
		this.lo = after;
		this.exhausted = over || overAfter;
		return !over && !lt(this.hi, item) ? item : DONE;
	}

	/** std's `nth_back`, from the end. */
	override nthBackItem(n: number): T | Done {
		if (this.isEmpty()) return DONE;
		const [item, over] = overflowing(this.hi, -n, this.bounds);
		const [before, overBefore] = overflowing(item, -1, this.bounds);
		this.hi = before;
		this.exhausted = over || overBefore;
		return !over && !lt(item, this.lo) ? item : DONE;
	}
}

/** An integer type's least and greatest values. */
interface Bounds<T> {
	readonly MIN: T;
	readonly MAX: T;
}

/** `Step::forward_overflowing` and `backward_overflowing`: `n + by`, wrapped within `bounds`, and whether it wrapped. */
function overflowing<T>(n: T, by: number, bounds: Bounds<T> | undefined): [T, boolean] {
	if (bounds === undefined) throw new Error("rust2-ts: stepping a range of no integer type");
	const exact = BigInt(n as number | bigint) + BigInt(by);
	const [min, max] = [BigInt(bounds.MIN as number | bigint), BigInt(bounds.MAX as number | bigint)];
	if (exact >= min && exact <= max) return [step(n, by), false];
	const span = max - min + 1n;
	const wrapped = ((((exact - min) % span) + span) % span) + min;
	return [(typeof n === "bigint" ? wrapped : Number(wrapped)) as T, true];
}

/** An iterator as itself: what `into_iter` makes of one. */
function itself<T>(iter: Iter<T>): Iter<T> {
	return iter;
}
