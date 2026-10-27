// Iterators, for rust2-ts output: std's `Iterator`, its adaptors, and `Range`, lazy as
// Rust's are. `rt.ts` re-exports all of it.
//
// `next` returns an item or `DONE`, never `undefined`, so an iterator of `Option<T>` keeps
// its `None` items apart from its end. Each adaptor calls what it wraps, and the closures
// it was given, in the order and as many times as std's does, and stops where std's
// stops: a `find` or `any` reads no further than its match. Adaptors and consumers are
// functions, `iterMap(it, f)`, so a bundler drops the ones nothing calls; an iterator's
// methods are only what each kind does its own way.
//
// An iterator that owns its items, as a `Vec`'s `into_iter` does, is given what drops
// each, and drops those it still holds as it drops, by `[Symbol.dispose]`; an adaptor
// drops what it wraps. Where std drops an item it skips, rejects, or replaces, the runtime
// function is given what drops it, after its own arguments, and so is a consumer that
// takes the iterator by value, which drops it as std's does, a panic included.

import { ByteVec, byteVecPush, Err, Ok, panic, type Result } from "../rt.ts";
import { type Limbs, lt as wideLt, type Step } from "./int.ts";
import { dropAll, dropRange } from "./task.ts";

/** What `next` returns once an iterator is exhausted, which no item is. */
export const DONE: unique symbol = /* @__PURE__ */ Symbol("DONE");

/** The type of `DONE`. */
export type Done = typeof DONE;

/** What drops a value. */
type Drop<T> = (value: T) => void;

/** `Iterator`: `next`, and the methods std's iterators each do their own way. */
export abstract class Iter<T> implements Iterable<T>, Disposable {
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

	/** `advance_by`: skip up to `n` items, each dropped by `drop`, and return how many it fell short. */
	advanceBy(n: number, drop?: Drop<T>): number {
		for (let i = 0; i < n; i++) {
			const item = this.next();
			if (item === DONE) return n - i;
			drop?.(item);
		}
		return 0;
	}

	/** `advance_back_by`, from the back. */
	advanceBackBy(n: number, drop?: Drop<T>): number {
		for (let i = 0; i < n; i++) {
			const item = this.nextBack();
			if (item === DONE) return n - i;
			drop?.(item);
		}
		return 0;
	}

	/** `nth`, as `next` returns it, dropping what it skips by `drop`. */
	nthItem(n: number, drop?: Drop<T>): T | Done {
		return this.advanceBy(n, drop) === 0 ? this.next() : DONE;
	}

	/** `nth_back`, as `next_back` returns it. */
	nthBackItem(n: number, drop?: Drop<T>): T | Done {
		return this.advanceBackBy(n, drop) === 0 ? this.nextBack() : DONE;
	}

	/** `count`, dropping each item by `drop`. */
	countItems(drop?: Drop<T>): number {
		let n = 0;
		for (let item = this.next(); item !== DONE; item = this.next()) {
			drop?.(item);
			n++;
		}
		return n;
	}

	/**
	 * `last`, as std's `fold` takes it: each item drops by `drop` as the next replaces it.
	 * A panic reading the next drops the last; one dropping it leaks the next, as std's
	 * unwinding leaves a returned value.
	 */
	lastItem(drop?: Drop<T>): T | Done {
		let last: T | Done = DONE;
		if (drop === undefined) {
			for (let item = this.next(); item !== DONE; item = this.next()) last = item;
			return last;
		}
		for (;;) {
			let item: T | Done;
			try {
				item = this.next();
			} catch (e) {
				if (last !== DONE) drop(last);
				throw e;
			}
			if (item === DONE) return last;
			const prev = last;
			last = item;
			if (prev !== DONE) drop(prev);
		}
	}

	/** Drop what it owns and still holds: nothing, unless it was made owning its items. */
	[Symbol.dispose](): void {}

	/** Its items, for a JS `for ... of`. */
	*[Symbol.iterator](): Iterator<T> {
		for (let item = this.next(); item !== DONE; item = this.next()) yield item;
	}
}

/** An adaptor of `inner`, which drops with it. */
abstract class Adaptor<S, T> extends Iter<T> {
	readonly inner: Iter<S>;

	constructor(inner: Iter<S>) {
		super();
		this.inner = inner;
	}

	override [Symbol.dispose](): void {
		this.inner[Symbol.dispose]();
	}
}

/** Run `a`, then `b` even if `a` panics, as glue drops each field after one panics. */
function both(a: () => void, b: () => void): void {
	try {
		a();
	} finally {
		b();
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

/** `slice::Iter`, and with what drops each item, what iterating a `Vec` or array by value makes. */
export function iterSlice<T>(s: ArrayLike<T>, drop?: Drop<T>): Iter<T> {
	return new SliceIter(s, drop);
}

/** `Option::iter`, and with what drops its item, `option::IntoIter`. */
export function iterOption<T>(o: T | undefined, drop?: Drop<T>): Iter<T> {
	return new OptionIter(o, o !== undefined, drop);
}

/** `iter::once`. */
export function iterOnce<T>(value: T, drop?: Drop<T>): Iter<T> {
	return new OptionIter<T>(value, true, drop);
}

/** `iter::empty`. */
export function iterEmpty<T>(): Iter<T> {
	return new OptionIter<T>(undefined, false);
}

/** `iter::from_fn`, whose closure returns an `Option`. */
export function iterFromFn<T>(f: () => T | undefined): Iter<T> {
	return new FromNext(f, (f) => iterFromOption(f()));
}

/** An `impl Iterator` of the translated crate, which `next` calls the `next` of, and `drop` drops. */
export function iterFrom<S, T>(source: S, next: (source: S) => T | Done, drop?: Drop<S>): Iter<T> {
	return new FromNext(source, next, drop);
}

/** An iterator borrowed by `&mut`, which std's adaptors take as one: its items, but never it to drop. */
export function iterBorrow<T>(it: Iter<T>): Iter<T> {
	return new Borrowed(it);
}

/** `Extend::extend` of a `Vec`, which drops `items` by `dropIt` once done or a panic. */
export function iterExtend<T>(v: T[], items: Iter<T>, dropIt?: Drop<Iter<T>>): void {
	if (dropIt === undefined) {
		for (let item = items.next(); item !== DONE; item = items.next()) v.push(item);
		return;
	}
	try {
		for (let item = items.next(); item !== DONE; item = items.next()) v.push(item);
	} finally {
		dropIt(items);
	}
}

/** `Extend::extend` of a `Vec<u8>`. */
export function iterExtendBytes(v: ByteVec, items: Iter<number>, dropIt?: Drop<Iter<number>>): void {
	try {
		for (let item = items.next(); item !== DONE; item = items.next()) byteVecPush(v, item);
	} finally {
		dropIt?.(items);
	}
}

// Adaptors

/** `Iterator::map`. */
export function iterMap<T, U>(it: Iter<T>, f: (item: T) => U): Iter<U> {
	return new MapIter(it, f);
}

/** `Iterator::filter`, dropping what `f` rejects by `drop`. */
export function iterFilter<T>(it: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>): Iter<T> {
	return new FilterIter(it, f, drop);
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
 * `zip`, with `other` already made an iterator, as std does when it is called, dropping
 * an item of either it reads without one of the other by `dropA` or `dropB`. A `trusted`
 * zip is one of two `TrustedRandomAccess` iterators, which std reads by index up to the
 * shorter length and never past it.
 */
export function iterZip<T, U>(it: Iter<T>, other: Iter<U>, trusted = false, dropA?: Drop<T>, dropB?: Drop<U>): Iter<[T, U]> {
	return trusted ? new TrustedZipIter(it, other) : new ZipIter(it, other, dropA, dropB);
}

/** `Iterator::rev`. */
export function iterRev<T>(it: Iter<T>): Iter<T> {
	return new RevIter(it);
}

/** `Iterator::skip`, dropping what it skips by `drop`. */
export function iterSkip<T>(it: Iter<T>, n: number, drop?: Drop<T>): Iter<T> {
	return new SkipIter(it, n, drop);
}

/** `Iterator::take`, dropping what `next_back` skips past its limit by `drop`. */
export function iterTake<T>(it: Iter<T>, n: number, drop?: Drop<T>): Iter<T> {
	return new TakeIter(it, n, drop);
}

/** `Iterator::skip_while`, dropping what it skips by `drop`. */
export function iterSkipWhile<T>(it: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>): Iter<T> {
	return new SkipWhileIter(it, f, drop);
}

/** `Iterator::take_while`, dropping the item that ends it by `drop`. */
export function iterTakeWhile<T>(it: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>): Iter<T> {
	return new TakeWhileIter(it, f, drop);
}

/** `Iterator::map_while`. */
export function iterMapWhile<T, U>(it: Iter<T>, f: (item: T) => U | undefined): Iter<U> {
	return new MapWhileIter(it, f);
}

/** `Iterator::step_by`, panicking on a step of 0, and dropping what it steps over by `drop`. */
export function iterStepBy<T>(it: Iter<T>, step: number, drop?: Drop<T>): Iter<T> {
	return new StepByIter(it, step, drop);
}

/** `Iterator::inspect`, dropping the item `f` panics on by `drop`. */
export function iterInspect<T>(it: Iter<T>, f: (item: T) => void, drop?: Drop<T>): Iter<T> {
	return new InspectIter(it, f, drop);
}

/**
 * `Iterator::fuse`, which drops `it` once it ends, unless it is `fused`, a
 * `FusedIterator`, which std's keeps.
 */
export function iterFuse<T>(it: Iter<T>, fused = false): Iter<T> {
	return new FuseIter(it, fused);
}

/** `Iterator::peekable`, which drops an item it peeked by `drop`. */
export function iterPeekable<T>(it: Iter<T>, drop?: Drop<T>): Peekable<T> {
	return new Peekable(it, drop);
}

/**
 * `flat_map`: `into` makes each of `f`'s results an iterator, as `IntoIterator` does. Its
 * source drops once it ends unless it is `fused`, as `fuse` does.
 */
export function iterFlatMap<T, V>(it: Iter<T>, f: (item: T) => Iter<V>): Iter<V>;
export function iterFlatMap<T, U, V>(it: Iter<T>, f: (item: T) => U, into?: (value: U) => Iter<V>, fused?: boolean): Iter<V>;
export function iterFlatMap<T, U, V>(it: Iter<T>, f: (item: T) => U, into?: (value: U) => Iter<V>, fused = false): Iter<V> {
	return new FlattenIter(new MapIter(it, f), into ?? (itself as (value: U) => Iter<V>), fused);
}

/** `flatten`: `into` makes each item an iterator, as `IntoIterator` does, and `fused` is as `flat_map`'s. */
export function iterFlatten<V>(it: Iter<Iter<V>>): Iter<V>;
export function iterFlatten<T, V>(it: Iter<T>, into?: (value: T) => Iter<V>, fused?: boolean): Iter<V>;
export function iterFlatten<T, V>(it: Iter<T>, into?: (value: T) => Iter<V>, fused = false): Iter<V> {
	return new FlattenIter(it, into ?? (itself as (value: T) => Iter<V>), fused);
}

// Consumers. Each that takes the iterator by value drops it by `dropIt` once done, or as
// a panic unwinds through it.

/** `Iterator::count`, dropping each item by `drop`. */
export function iterCount<T>(it: Iter<T>, dropIt?: Drop<Iter<T>>, drop?: Drop<T>): number {
	if (dropIt === undefined) return it.countItems(drop);
	try {
		return it.countItems(drop);
	} finally {
		dropIt(it);
	}
}

/** `Iterator::last`, dropping each item but the last by `drop`. */
export function iterLast<T>(it: Iter<T>, dropIt?: Drop<Iter<T>>, drop?: Drop<T>): T | undefined {
	if (dropIt === undefined) return iterToOption(it.lastItem(drop));
	let last: T | Done;
	try {
		last = it.lastItem(drop);
	} catch (e) {
		dropIt(it);
		throw e;
	}
	// A panic here leaks the last, as std's unwinding does a returned value.
	dropIt(it);
	return iterToOption(last);
}

/** `Iterator::nth`, dropping what it skips by `drop`. */
export function iterNth<T>(it: Iter<T>, n: number, drop?: Drop<T>): T | undefined {
	return iterToOption(it.nthItem(n, drop));
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

/** `find`, as `next` returns it: what `f` rejects, or panics on, drops by `drop`. */
function findItem<T>(it: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>): T | Done {
	if (drop === undefined) {
		for (let item = it.next(); item !== DONE; item = it.next()) if (f(item)) return item;
		return DONE;
	}
	for (let item = it.next(); item !== DONE; item = it.next()) {
		if (test(f, item, drop)) return item;
		drop(item);
	}
	return DONE;
}

/** `rfind`, as `next_back` returns it. */
function rfindItem<T>(it: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>): T | Done {
	for (let item = it.nextBack(); item !== DONE; item = it.nextBack()) {
		if (drop === undefined ? f(item) : test(f, item, drop)) return item;
		drop?.(item);
	}
	return DONE;
}

/** `f(item)`, dropping `item` by `drop` if it panics. */
function test<T>(f: (item: T) => boolean, item: T, drop: Drop<T>): boolean {
	try {
		return f(item);
	} catch (e) {
		drop(item);
		throw e;
	}
}

/** `Iterator::find`, dropping what `f` rejects by `drop`. */
export function iterFind<T>(it: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>): T | undefined {
	return iterToOption(findItem(it, f, drop));
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

/**
 * `Iterator::fold`. A panic reading an item drops the accumulator by `dropAcc`; one in
 * `f`, which took it, leaves it to `f`.
 */
export function iterFold<T, A>(it: Iter<T>, init: A, f: (acc: A, item: T) => A, dropIt?: Drop<Iter<T>>, dropAcc?: Drop<A>): A {
	let acc = init;
	if (dropIt === undefined && dropAcc === undefined) {
		for (let item = it.next(); item !== DONE; item = it.next()) acc = f(acc, item);
		return acc;
	}
	try {
		for (;;) {
			let item: T | Done;
			try {
				item = it.next();
			} catch (e) {
				dropAcc?.(acc);
				throw e;
			}
			if (item === DONE) break;
			acc = f(acc, item);
		}
	} catch (e) {
		dropIt?.(it);
		throw e;
	}
	// A panic here leaks the accumulator, as std's unwinding does a returned value.
	dropIt?.(it);
	return acc;
}

/** `Iterator::for_each`. */
export function iterForEach<T>(it: Iter<T>, f: (item: T) => void, dropIt?: Drop<Iter<T>>): void {
	if (dropIt === undefined) {
		for (let item = it.next(); item !== DONE; item = it.next()) f(item);
		return;
	}
	try {
		for (let item = it.next(); item !== DONE; item = it.next()) f(item);
	} finally {
		dropIt(it);
	}
}

/** `sum`, adding with an integer type's checked `add`, so overflow panics as Rust's does. */
export function iterSum<T>(it: Iter<T>, add: (a: T, b: T) => T, zero: T, dropIt?: Drop<Iter<T>>): T {
	return iterFold(it, zero, add, dropIt);
}

/** `product`, multiplying with an integer type's checked `mul`. */
export function iterProduct<T>(it: Iter<T>, mul: (a: T, b: T) => T, one: T, dropIt?: Drop<Iter<T>>): T {
	return iterFold(it, one, mul, dropIt);
}

/** `max` of integers or booleans: the last of the greatest, as `Ord::max` keeps it. */
export function iterMax<K extends number | Limbs | boolean>(it: Iter<K>, dropIt?: Drop<Iter<K>>): K | undefined {
	return iterMaxByKey(it, (item) => item, dropIt);
}

/** `min` of integers or booleans: the first of the least. */
export function iterMin<K extends number | Limbs | boolean>(it: Iter<K>, dropIt?: Drop<Iter<K>>): K | undefined {
	return iterMinByKey(it, (item) => item, dropIt);
}

/** `max_by_key` with an integer or boolean key, each item's taken once, dropping the others by `drop`. */
export function iterMaxByKey<T, K extends number | Limbs | boolean>(it: Iter<T>, f: (item: T) => K, dropIt?: Drop<Iter<T>>, drop?: Drop<T>): T | undefined {
	return best(it, f, false, dropIt, drop);
}

/** `min_by_key` with an integer or boolean key, each item's taken once, dropping the others by `drop`. */
export function iterMinByKey<T, K extends number | Limbs | boolean>(it: Iter<T>, f: (item: T) => K, dropIt?: Drop<Iter<T>>, drop?: Drop<T>): T | undefined {
	return best(it, f, true, dropIt, drop);
}

/**
 * The item of the greatest key, the last of equal ones, or of the least if `least`, the
 * first, as std's `reduce` keeps it: each other drops as it loses. A panic in `f` drops
 * its item, then the best, then the iterator; one dropping a loser leaks the winner, as
 * std's unwinding does a returned value.
 */
function best<T, K extends number | Limbs | boolean>(it: Iter<T>, f: (item: T) => K, least: boolean, dropIt?: Drop<Iter<T>>, drop?: Drop<T>): T | undefined {
	if (dropIt === undefined && drop === undefined) {
		let best = it.next();
		if (best === DONE) return undefined;
		let key = f(best);
		for (let item = it.next(); item !== DONE; item = it.next()) {
			const k = f(item);
			if (least ? lt(k, key) : !lt(k, key)) [best, key] = [item, k];
		}
		return best;
	}
	let best: T | Done = DONE;
	let key: K | undefined;
	try {
		for (let item = it.next(); item !== DONE; item = it.next()) {
			let k: K;
			try {
				k = f(item);
			} catch (e) {
				drop?.(item);
				throw e;
			}
			if (best === DONE) {
				[best, key] = [item, k];
				continue;
			}
			const wins = least ? lt(k, key as K) : !lt(k, key as K);
			const loser = wins ? best : item;
			if (wins) [best, key] = [item, k];
			if (drop !== undefined) {
				const held: T | Done = best;
				best = DONE;
				drop(loser);
				best = held;
			}
		}
	} catch (e) {
		try {
			if (best !== DONE) drop?.(best);
		} finally {
			dropIt?.(it);
		}
		throw e;
	}
	// A panic here leaks the best, as std's unwinding does a returned value.
	dropIt?.(it);
	return iterToOption(best);
}

/**
 * `collect` into a `Vec`. A panic drops `it` by `dropIt` and then what it collected by
 * `drop`, or with `inPlace`, as std's collects into the source `Vec`'s buffer, the other
 * way around.
 */
export function iterToArray<T>(it: Iter<T>, inPlace = false, dropIt?: Drop<Iter<T>>, drop?: Drop<T>): T[] {
	const out: T[] = [];
	if (dropIt === undefined && drop === undefined) {
		for (let item = it.next(); item !== DONE; item = it.next()) out.push(item);
		return out;
	}
	try {
		for (let item = it.next(); item !== DONE; item = it.next()) out.push(item);
	} catch (e) {
		if (inPlace) both(() => dropAll(out, drop), () => dropIt?.(it));
		else both(() => dropIt?.(it), () => dropAll(out, drop));
		throw e;
	}
	try {
		dropIt?.(it);
	} catch (e) {
		dropAll(out, drop);
		throw e;
	}
	return out;
}

/** `collect` into a `Vec<u8>`. */
export function iterToByteVec(it: Iter<number>, dropIt?: Drop<Iter<number>>): ByteVec {
	const out = new ByteVec();
	// Its own loop: through `iterExtendBytes`, JavaScriptCore took 1.9x as long.
	try {
		for (let item = it.next(); item !== DONE; item = it.next()) byteVecPush(out, item);
	} finally {
		dropIt?.(it);
	}
	return out;
}

/**
 * `collect` of `Result`s: the first error, or every value collected by `into`, which drops
 * the iterator. On an error, what it collected drops by `drop`; on a panic after one, the
 * error drops by `dropErr`.
 */
export function iterCollectResult<U, E, C>(it: Iter<Result<U, E>>, into: (values: Iter<U>) => C, drop?: Drop<C>, dropErr?: Drop<E>): Result<C, E> {
	const shunt = new ResultShunt(it);
	let values: C;
	try {
		values = into(shunt);
	} catch (e) {
		if (shunt.error !== undefined) dropErr?.(shunt.error.value);
		throw e;
	}
	if (shunt.error === undefined) return Ok(values);
	const error = Err(shunt.error.value);
	drop?.(values);
	return error;
}

/** `collect` of `Option`s: `None` at the first `None`, or every value collected by `into`, as `iterCollectResult`. */
export function iterCollectOption<U, C>(it: Iter<U | undefined>, into: (values: Iter<U>) => C, drop?: Drop<C>): C | undefined {
	const shunt = new OptionShunt(it);
	const values = into(shunt);
	if (!shunt.none) return values;
	drop?.(values);
	return undefined;
}

class SliceIter<T> extends Iter<T> {
	private readonly s: ArrayLike<T>;
	private front = 0;
	private back: number;
	/** What drops each item it still holds, if it owns them. */
	private readonly drop: Drop<T> | undefined;

	constructor(s: ArrayLike<T>, drop?: Drop<T>) {
		super();
		this.s = s;
		this.back = s.length;
		this.drop = drop;
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

	/** std's, of a `Vec`'s: past them all first, then each dropped, as a slice drops. */
	override advanceBy(n: number, drop?: Drop<T>): number {
		const step = Math.min(n, this.back - this.front);
		const from = this.front;
		this.front += step;
		if (drop !== undefined) dropRange(this.s, drop, from, this.front);
		return n - step;
	}

	/** std's: those it skips drop front to back, as a slice drops. */
	override advanceBackBy(n: number, drop?: Drop<T>): number {
		const step = Math.min(n, this.back - this.front);
		this.back -= step;
		if (drop !== undefined) dropRange(this.s, drop, this.back, this.back + step);
		return n - step;
	}

	/** std's, of a `Vec`'s or an array's: the last item, then the rest drop with it. */
	override lastItem(drop?: Drop<T>): T | Done {
		if (drop === undefined) return super.lastItem();
		const last = this.nextBack();
		// A panic here leaks the last, as std's unwinding does a returned value.
		this[Symbol.dispose]();
		return last;
	}

	override [Symbol.dispose](): void {
		if (this.drop === undefined) return;
		const from = this.front;
		this.front = this.back;
		dropRange(this.s, this.drop, from, this.back);
	}
}

/** `option::IntoIter`, `Once`, and `Empty`: at most one item. */
class OptionIter<T> extends Iter<T> {
	private item: T | undefined;
	private some: boolean;
	private readonly drop: Drop<T> | undefined;

	constructor(item: T | undefined, some: boolean, drop?: Drop<T>) {
		super();
		this.item = item;
		this.some = some;
		this.drop = drop;
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

	override [Symbol.dispose](): void {
		if (this.drop === undefined) return;
		const item = this.next();
		if (item !== DONE) this.drop(item);
	}
}

/** An iterator whose `next` is a function of its source, which `drop` drops. */
class FromNext<S, T> extends Iter<T> {
	private readonly source: S;
	private readonly step: (source: S) => T | Done;
	private drop: Drop<S> | undefined;

	constructor(source: S, step: (source: S) => T | Done, drop?: Drop<S>) {
		super();
		this.source = source;
		this.step = step;
		this.drop = drop;
	}

	next(): T | Done {
		return this.step(this.source);
	}

	override [Symbol.dispose](): void {
		const drop = this.drop;
		this.drop = undefined;
		drop?.(this.source);
	}
}

/** An iterator borrowed by `&mut`: dropping it drops nothing. */
class Borrowed<T> extends Adaptor<T, T> {
	next(): T | Done {
		return this.inner.next();
	}

	override nextBack(): T | Done {
		return this.inner.nextBack();
	}

	override len(): number {
		return this.inner.len();
	}

	override advanceBy(n: number, drop?: Drop<T>): number {
		return this.inner.advanceBy(n, drop);
	}

	override advanceBackBy(n: number, drop?: Drop<T>): number {
		return this.inner.advanceBackBy(n, drop);
	}

	override nthItem(n: number, drop?: Drop<T>): T | Done {
		return this.inner.nthItem(n, drop);
	}

	override nthBackItem(n: number, drop?: Drop<T>): T | Done {
		return this.inner.nthBackItem(n, drop);
	}

	override [Symbol.dispose](): void {}
}

class MapIter<T, U> extends Adaptor<T, U> {
	private readonly f: (item: T) => U;

	constructor(inner: Iter<T>, f: (item: T) => U) {
		super(inner);
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

class FilterIter<T> extends Adaptor<T, T> {
	private readonly f: (item: T) => boolean;
	private readonly drop: Drop<T> | undefined;

	constructor(inner: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>) {
		super(inner);
		this.f = f;
		this.drop = drop;
	}

	next(): T | Done {
		return findItem(this.inner, this.f, this.drop);
	}

	override nextBack(): T | Done {
		return rfindItem(this.inner, this.f, this.drop);
	}
}

class FilterMapIter<T, U> extends Adaptor<T, U> {
	private readonly f: (item: T) => U | undefined;

	constructor(inner: Iter<T>, f: (item: T) => U | undefined) {
		super(inner);
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

class EnumerateIter<T> extends Adaptor<T, [number, T]> {
	private index = 0;

	next(): [number, T] | Done {
		const item = this.inner.next();
		return item === DONE ? DONE : [this.index++, item];
	}

	override nextBack(): [number, T] | Done {
		const item = this.inner.nextBack();
		return item === DONE ? DONE : [this.index + this.inner.len(), item];
	}

	/** std's, which counts what its iterator skipped. */
	override advanceBy(n: number, drop?: Drop<[number, T]>): number {
		const left = this.inner.advanceBy(n, items(drop));
		this.index += n - left;
		return left;
	}

	override advanceBackBy(n: number, drop?: Drop<[number, T]>): number {
		return this.inner.advanceBackBy(n, items(drop));
	}

	/** std's `nth`, which counts only when it finds an item. */
	override nthItem(n: number, drop?: Drop<[number, T]>): [number, T] | Done {
		const item = this.inner.nthItem(n, items(drop));
		if (item === DONE) return DONE;
		const i = this.index + n;
		this.index = i + 1;
		return [i, item];
	}

	override nthBackItem(n: number, drop?: Drop<[number, T]>): [number, T] | Done {
		const item = this.inner.nthBackItem(n, items(drop));
		return item === DONE ? DONE : [this.index + this.inner.len(), item];
	}

	override countItems(drop?: Drop<[number, T]>): number {
		return this.inner.countItems(items(drop));
	}

	override len(): number {
		return this.inner.len();
	}
}

/** What drops an `enumerate`'s item, of one its iterator yields. */
function items<T>(drop: Drop<[number, T]> | undefined): Drop<T> | undefined {
	return drop === undefined ? undefined : (item) => drop([0, item]);
}

/**
 * `Chain`, which drops its first iterator once it ends, but not its second, as std's
 * stops reading it.
 */
class ChainIter<T> extends Iter<T> {
	private a: Iter<T> | undefined;
	private b: Iter<T> | undefined;

	constructor(a: Iter<T>, b: Iter<T>) {
		super();
		this.a = a;
		this.b = b;
	}

	/** Drop `a`, which ended. */
	private endA(): void {
		const a = this.a;
		this.a = undefined;
		a?.[Symbol.dispose]();
	}

	/** Drop `b`, which ended from the back. */
	private endB(): void {
		const b = this.b;
		this.b = undefined;
		b?.[Symbol.dispose]();
	}

	next(): T | Done {
		if (this.a !== undefined) {
			const item = this.a.next();
			if (item !== DONE) return item;
			this.endA();
		}
		return this.b === undefined ? DONE : this.b.next();
	}

	override nextBack(): T | Done {
		if (this.b !== undefined) {
			const item = this.b.nextBack();
			if (item !== DONE) return item;
			this.endB();
		}
		return this.a === undefined ? DONE : this.a.nextBack();
	}

	override advanceBy(n: number, drop?: Drop<T>): number {
		if (this.a !== undefined) {
			n = this.a.advanceBy(n, drop);
			if (n === 0) return 0;
			this.endA();
		}
		return this.b === undefined ? n : this.b.advanceBy(n, drop);
	}

	override advanceBackBy(n: number, drop?: Drop<T>): number {
		if (this.b !== undefined) {
			n = this.b.advanceBackBy(n, drop);
			if (n === 0) return 0;
			this.endB();
		}
		return this.a === undefined ? n : this.a.advanceBackBy(n, drop);
	}

	override nthItem(n: number, drop?: Drop<T>): T | Done {
		if (this.a !== undefined) {
			n = this.a.advanceBy(n, drop);
			if (n === 0) {
				const item = this.a.next();
				if (item !== DONE) return item;
			}
			this.endA();
		}
		return this.b === undefined ? DONE : this.b.nthItem(n, drop);
	}

	override nthBackItem(n: number, drop?: Drop<T>): T | Done {
		if (this.b !== undefined) {
			n = this.b.advanceBackBy(n, drop);
			if (n === 0) {
				const item = this.b.nextBack();
				if (item !== DONE) return item;
			}
			this.endB();
		}
		return this.a === undefined ? DONE : this.a.nthBackItem(n, drop);
	}

	/** std's: each iterator's own `count`, the first's first. */
	override countItems(drop?: Drop<T>): number {
		const a = this.a;
		this.a = undefined;
		const n = a === undefined ? 0 : consumed(a, () => a.countItems(drop));
		const b = this.b;
		this.b = undefined;
		return n + (b === undefined ? 0 : consumed(b, () => b.countItems(drop)));
	}

	/**
	 * std's: each iterator's last, the first's held while the second is read, and dropped
	 * once the second has one.
	 */
	override lastItem(drop?: Drop<T>): T | Done {
		const a = this.a;
		this.a = undefined;
		const aLast = a === undefined ? DONE : consumed(a, () => a.lastItem(drop));
		const b = this.b;
		this.b = undefined;
		if (b === undefined) return aLast;
		let bLast: T | Done;
		try {
			bLast = consumed(b, () => b.lastItem(drop));
		} catch (e) {
			if (aLast !== DONE) drop?.(aLast);
			throw e;
		}
		if (bLast === DONE) return aLast;
		// A panic here leaks the second's, as std's unwinding does a returned value.
		if (aLast !== DONE) drop?.(aLast);
		return bLast;
	}

	override [Symbol.dispose](): void {
		both(
			() => this.endA(),
			() => this.endB(),
		);
	}
}

/**
 * `f()`, a consumer of `it` by value, and then `it` dropped, as a panic unwinds too; a
 * panic dropping it leaks what `f` returned, as std's unwinding does.
 */
function consumed<T, R>(it: Iter<T>, f: () => R): R {
	let r: R;
	try {
		r = f();
	} catch (e) {
		it[Symbol.dispose]();
		throw e;
	}
	it[Symbol.dispose]();
	return r;
}

class ZipIter<A, B> extends Iter<[A, B]> {
	protected readonly a: Iter<A>;
	protected readonly b: Iter<B>;
	private readonly dropA: Drop<A> | undefined;
	private readonly dropB: Drop<B> | undefined;

	constructor(a: Iter<A>, b: Iter<B>, dropA?: Drop<A>, dropB?: Drop<B>) {
		super();
		this.a = a;
		this.b = b;
		this.dropA = dropA;
		this.dropB = dropB;
	}

	/** An item of `a` it read, dropped if reading `b` panics or ends. */
	next(): [A, B] | Done {
		const a = this.a.next();
		if (a === DONE) return DONE;
		let b: B | Done;
		try {
			b = this.b.next();
		} catch (e) {
			this.dropA?.(a);
			throw e;
		}
		if (b !== DONE) return [a, b];
		this.dropA?.(a);
		return DONE;
	}

	/** std's: the longer side drops its extra items from the back first. */
	override nextBack(): [A, B] | Done {
		const aLen = this.a.len();
		const bLen = this.b.len();
		for (let i = bLen; i < aLen; i++) {
			const item = this.a.nextBack();
			if (item !== DONE) this.dropA?.(item);
		}
		for (let i = aLen; i < bLen; i++) {
			const item = this.b.nextBack();
			if (item !== DONE) this.dropB?.(item);
		}
		const a = this.a.nextBack();
		let b: B | Done;
		try {
			b = this.b.nextBack();
		} catch (e) {
			if (a !== DONE) this.dropA?.(a);
			throw e;
		}
		if (a === DONE && b === DONE) return DONE;
		if (a === DONE || b === DONE) panic("internal error: entered unreachable code");
		return [a, b];
	}

	override len(): number {
		return Math.min(this.a.len(), this.b.len());
	}

	override [Symbol.dispose](): void {
		both(
			() => this.a[Symbol.dispose](),
			() => this.b[Symbol.dispose](),
		);
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

class RevIter<T> extends Adaptor<T, T> {
	next(): T | Done {
		return this.inner.nextBack();
	}

	override nextBack(): T | Done {
		return this.inner.next();
	}

	override advanceBy(n: number, drop?: Drop<T>): number {
		return this.inner.advanceBackBy(n, drop);
	}

	override advanceBackBy(n: number, drop?: Drop<T>): number {
		return this.inner.advanceBy(n, drop);
	}

	override nthItem(n: number, drop?: Drop<T>): T | Done {
		return this.inner.nthBackItem(n, drop);
	}

	override nthBackItem(n: number, drop?: Drop<T>): T | Done {
		return this.inner.nthItem(n, drop);
	}

	override len(): number {
		return this.inner.len();
	}
}

class SkipIter<T> extends Adaptor<T, T> {
	private n: number;
	private readonly drop: Drop<T> | undefined;

	constructor(inner: Iter<T>, n: number, drop?: Drop<T>) {
		super(inner);
		this.n = n;
		this.drop = drop;
	}

	next(): T | Done {
		if (this.n === 0) return this.inner.next();
		const n = this.n;
		this.n = 0;
		return this.inner.nthItem(n, this.drop);
	}

	override nextBack(): T | Done {
		return this.len() > 0 ? this.inner.nextBack() : DONE;
	}

	/** std's: what it still skips and then `n` more, in one step. */
	override nthItem(n: number, drop?: Drop<T>): T | Done {
		const skip = this.n;
		this.n = 0;
		return this.inner.nthItem(skip + n, drop);
	}

	override advanceBy(n: number, drop?: Drop<T>): number {
		const skip = this.n;
		const left = this.inner.advanceBy(skip + n, drop);
		const advanced = skip + n - left;
		this.n = Math.max(skip - advanced, 0);
		return n - Math.max(advanced - skip, 0);
	}

	/** std's: past what it skips by `nth`, and then its iterator's own `count`. */
	override countItems(drop?: Drop<T>): number {
		if (this.skipped(drop)) return 0;
		return consumed(this.inner, () => this.inner.countItems(drop));
	}

	/** std's: past what it skips by `nth`, and then its iterator's own `last`. */
	override lastItem(drop?: Drop<T>): T | Done {
		if (this.skipped(drop)) return DONE;
		return consumed(this.inner, () => this.inner.lastItem(drop));
	}

	/** Read past what it still skips, as `nth` reads, and whether its iterator ended first. */
	private skipped(drop: Drop<T> | undefined): boolean {
		if (this.n === 0) return false;
		const n = this.n;
		this.n = 0;
		const item = this.inner.nthItem(n - 1, drop);
		if (item === DONE) return true;
		drop?.(item);
		return false;
	}

	override len(): number {
		return Math.max(this.inner.len() - this.n, 0);
	}
}

class TakeIter<T> extends Adaptor<T, T> {
	private n: number;
	private readonly drop: Drop<T> | undefined;

	constructor(inner: Iter<T>, n: number, drop?: Drop<T>) {
		super(inner);
		this.n = n;
		this.drop = drop;
	}

	next(): T | Done {
		if (this.n === 0) return DONE;
		this.n--;
		return this.inner.next();
	}

	/** std's: what its iterator holds past the limit drops as it reads from the back. */
	override nextBack(): T | Done {
		if (this.n === 0) return DONE;
		const n = this.n--;
		return this.inner.nthBackItem(Math.max(this.inner.len() - n, 0), this.drop);
	}

	/** std's `nth`: past the limit, it reads the rest of the limit through, drops it, and ends. */
	override nthItem(n: number, drop?: Drop<T>): T | Done {
		if (this.n > n) {
			this.n -= n + 1;
			return this.inner.nthItem(n, drop);
		}
		if (this.n > 0) {
			const item = this.inner.nthItem(this.n - 1, drop);
			if (item !== DONE) drop?.(item);
			this.n = 0;
		}
		return DONE;
	}

	/** std's `nth_back`: past the limit, it reads the whole rest from the back, even at a limit of 0. */
	override nthBackItem(n: number, drop?: Drop<T>): T | Done {
		const len = this.inner.len();
		if (this.n > n) {
			const m = Math.max(len - this.n, 0) + n;
			this.n -= n + 1;
			return this.inner.nthBackItem(m, drop);
		}
		if (len > 0) {
			const item = this.inner.nthBackItem(len - 1, drop);
			if (item !== DONE) drop?.(item);
		}
		return DONE;
	}

	/** std's: its iterator advanced through the limit, dropping what it passes. */
	override countItems(drop?: Drop<T>): number {
		if (this.n === 0) return 0;
		const n = this.n;
		this.n = 0;
		return n - this.inner.advanceBy(n, drop);
	}

	override advanceBy(n: number, drop?: Drop<T>): number {
		const min = Math.min(this.n, n);
		const advanced = min - this.inner.advanceBy(min, drop);
		this.n -= advanced;
		return n - advanced;
	}

	override len(): number {
		return Math.min(this.inner.len(), this.n);
	}
}

class SkipWhileIter<T> extends Adaptor<T, T> {
	private readonly f: (item: T) => boolean;
	private readonly drop: Drop<T> | undefined;
	private flag = false;

	constructor(inner: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>) {
		super(inner);
		this.f = f;
		this.drop = drop;
	}

	next(): T | Done {
		return findItem(
			this.inner,
			(item) => {
				if (this.flag || !this.f(item)) {
					this.flag = true;
					return true;
				}
				return false;
			},
			this.drop,
		);
	}
}

class TakeWhileIter<T> extends Adaptor<T, T> {
	private readonly f: (item: T) => boolean;
	private readonly drop: Drop<T> | undefined;
	private flag = false;

	constructor(inner: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>) {
		super(inner);
		this.f = f;
		this.drop = drop;
	}

	next(): T | Done {
		if (this.flag) return DONE;
		const item = this.inner.next();
		if (item === DONE) return DONE;
		if (this.drop === undefined ? this.f(item) : test(this.f, item, this.drop)) return item;
		this.flag = true;
		this.drop?.(item);
		return DONE;
	}
}

/** `MapWhile`, which reads on after `f` returns `None` if called again, as std's does. */
class MapWhileIter<T, U> extends Adaptor<T, U> {
	private readonly f: (item: T) => U | undefined;

	constructor(inner: Iter<T>, f: (item: T) => U | undefined) {
		super(inner);
		this.f = f;
	}

	next(): U | Done {
		const item = this.inner.next();
		return item === DONE ? DONE : iterFromOption(this.f(item));
	}
}

class StepByIter<T> extends Adaptor<T, T> {
	private readonly step: number;
	private readonly drop: Drop<T> | undefined;
	private firstTake = true;

	constructor(inner: Iter<T>, step: number, drop?: Drop<T>) {
		super(inner);
		// The iterator it was given drops as the panic unwinds.
		if (step === 0) {
			inner[Symbol.dispose]();
			panic("assertion failed: step != 0");
		}
		this.step = step - 1;
		this.drop = drop;
	}

	next(): T | Done {
		const n = this.firstTake ? 0 : this.step;
		this.firstTake = false;
		return this.inner.nthItem(n, this.drop);
	}

	override nextBack(): T | Done {
		const rem = this.inner.len() % (this.step + 1);
		const n = this.firstTake ? (rem === 0 ? this.step : rem - 1) : rem;
		return this.inner.nthBackItem(n, this.drop);
	}

	override len(): number {
		const n = this.inner.len();
		if (this.firstTake) return n === 0 ? 0 : 1 + Math.floor((n - 1) / (this.step + 1));
		return Math.floor(n / (this.step + 1));
	}
}

class InspectIter<T> extends Adaptor<T, T> {
	private readonly f: (item: T) => void;
	private readonly drop: Drop<T> | undefined;

	constructor(inner: Iter<T>, f: (item: T) => void, drop?: Drop<T>) {
		super(inner);
		this.f = f;
		this.drop = drop;
	}

	next(): T | Done {
		return this.inspect(this.inner.next());
	}

	override nextBack(): T | Done {
		return this.inspect(this.inner.nextBack());
	}

	/** `f` of `item`, which drops if `f` panics. */
	private inspect(item: T | Done): T | Done {
		if (item === DONE) return item;
		if (this.drop === undefined) this.f(item);
		else test((item) => (this.f(item), true), item, this.drop);
		return item;
	}

	override len(): number {
		return this.inner.len();
	}
}

class FuseIter<T> extends Iter<T> {
	private inner: Iter<T> | undefined;
	/** Whether its iterator is a `FusedIterator`, which std's keeps once it ends. */
	private readonly fused: boolean;

	constructor(inner: Iter<T>, fused: boolean) {
		super();
		this.inner = inner;
		this.fused = fused;
	}

	/** `item`, dropping its iterator first if it ended there and is not fused. */
	private clear(item: T | Done): T | Done {
		if (item === DONE && !this.fused) {
			const inner = this.inner;
			this.inner = undefined;
			inner?.[Symbol.dispose]();
		}
		return item;
	}

	next(): T | Done {
		return this.inner === undefined ? DONE : this.clear(this.inner.next());
	}

	override nextBack(): T | Done {
		return this.inner === undefined ? DONE : this.clear(this.inner.nextBack());
	}

	override nthItem(n: number, drop?: Drop<T>): T | Done {
		return this.inner === undefined ? DONE : this.clear(this.inner.nthItem(n, drop));
	}

	override nthBackItem(n: number, drop?: Drop<T>): T | Done {
		return this.inner === undefined ? DONE : this.clear(this.inner.nthBackItem(n, drop));
	}

	override advanceBy(n: number, drop?: Drop<T>): number {
		if (this.inner === undefined) return n;
		const left = this.inner.advanceBy(n, drop);
		if (left > 0) this.clear(DONE);
		return left;
	}

	override advanceBackBy(n: number, drop?: Drop<T>): number {
		if (this.inner === undefined) return n;
		const left = this.inner.advanceBackBy(n, drop);
		if (left > 0) this.clear(DONE);
		return left;
	}

	override countItems(drop?: Drop<T>): number {
		const inner = this.inner;
		this.inner = undefined;
		return inner === undefined ? 0 : consumed(inner, () => inner.countItems(drop));
	}

	override lastItem(drop?: Drop<T>): T | Done {
		const inner = this.inner;
		this.inner = undefined;
		return inner === undefined ? DONE : consumed(inner, () => inner.lastItem(drop));
	}

	override len(): number {
		return this.inner === undefined ? 0 : this.inner.len();
	}

	override [Symbol.dispose](): void {
		const inner = this.inner;
		this.inner = undefined;
		inner?.[Symbol.dispose]();
	}
}

/**
 * `Flatten` and `FlatMap`: the iterator of each item it reads, which drops once it ends,
 * as does its source unless it is `fused`.
 */
class FlattenIter<T, V> extends Iter<V> {
	private outer: Iter<T> | undefined;
	private readonly into: (value: T) => Iter<V>;
	private readonly fused: boolean;
	private front: Iter<V> | undefined;
	private back: Iter<V> | undefined;

	constructor(outer: Iter<T>, into: (value: T) => Iter<V>, fused: boolean) {
		super();
		this.outer = outer;
		this.into = into;
		this.fused = fused;
	}

	/** The source's next item, or `DONE`, which drops it unless it is fused. */
	private nextOuter(back: boolean): T | Done {
		const outer = this.outer;
		if (outer === undefined) return DONE;
		const item = back ? outer.nextBack() : outer.next();
		if (item === DONE && !this.fused) {
			this.outer = undefined;
			outer[Symbol.dispose]();
		}
		return item;
	}

	/** `it`'s next item, from the back if `back`, dropping it if it ended. */
	private static step<V>(it: Iter<V>, back: boolean): V | Done {
		const item = back ? it.nextBack() : it.next();
		if (item === DONE) it[Symbol.dispose]();
		return item;
	}

	next(): V | Done {
		for (;;) {
			if (this.front !== undefined) {
				const item = FlattenIter.step(this.front, false);
				if (item !== DONE) return item;
				this.front = undefined;
			}
			const inner = this.nextOuter(false);
			if (inner === DONE) {
				if (this.back === undefined) return DONE;
				const item = FlattenIter.step(this.back, false);
				if (item === DONE) this.back = undefined;
				return item;
			}
			this.front = this.into(inner);
		}
	}

	override nextBack(): V | Done {
		for (;;) {
			if (this.back !== undefined) {
				const item = FlattenIter.step(this.back, true);
				if (item !== DONE) return item;
				this.back = undefined;
			}
			const inner = this.nextOuter(true);
			if (inner === DONE) {
				if (this.front === undefined) return DONE;
				const item = FlattenIter.step(this.front, true);
				if (item === DONE) this.front = undefined;
				return item;
			}
			this.back = this.into(inner);
		}
	}

	/** Hold `it` as the front iterator, dropping the one it held, as std's `insert` does. */
	private setFront(it: Iter<V> | undefined): void {
		const old = this.front;
		this.front = it;
		old?.[Symbol.dispose]();
	}

	private setBack(it: Iter<V> | undefined): void {
		const old = this.back;
		this.back = it;
		old?.[Symbol.dispose]();
	}

	/**
	 * std's, by `try_fold`: each iterator in turn advanced by its own `advance_by`, one that
	 * falls short dropped only as the next replaces it, and the source dropped once it ends
	 * unless it is fused.
	 */
	override advanceBy(n: number, drop?: Drop<V>): number {
		if (this.front !== undefined) {
			n = this.front.advanceBy(n, drop);
			if (n === 0) return 0;
		}
		this.setFront(undefined);
		const outer = this.outer;
		if (outer !== undefined) {
			for (let inner = outer.next(); inner !== DONE; inner = outer.next()) {
				const it = this.into(inner);
				this.setFront(it);
				n = it.advanceBy(n, drop);
				if (n === 0) return 0;
			}
			if (!this.fused) {
				this.outer = undefined;
				outer[Symbol.dispose]();
			}
		}
		this.setFront(undefined);
		if (this.back !== undefined) {
			n = this.back.advanceBy(n, drop);
			if (n === 0) return 0;
		}
		this.setBack(undefined);
		return n;
	}

	override advanceBackBy(n: number, drop?: Drop<V>): number {
		if (this.back !== undefined) {
			n = this.back.advanceBackBy(n, drop);
			if (n === 0) return 0;
		}
		this.setBack(undefined);
		const outer = this.outer;
		if (outer !== undefined) {
			for (let inner = outer.nextBack(); inner !== DONE; inner = outer.nextBack()) {
				const it = this.into(inner);
				this.setBack(it);
				n = it.advanceBackBy(n, drop);
				if (n === 0) return 0;
			}
			if (!this.fused) {
				this.outer = undefined;
				outer[Symbol.dispose]();
			}
		}
		this.setBack(undefined);
		if (this.front !== undefined) {
			n = this.front.advanceBackBy(n, drop);
			if (n === 0) return 0;
		}
		this.setFront(undefined);
		return n;
	}

	/** std's: each iterator's own `count` in turn, each dropped once counted. */
	override countItems(drop?: Drop<V>): number {
		let n = 0;
		const count = (it: Iter<V>): void => {
			n += consumed(it, () => it.countItems(drop));
		};
		this.consume(count);
		return n;
	}

	/**
	 * Run `f` on each iterator in turn, the front, each of the source's, and the back, as
	 * std's `fold` takes them by value; the source drops once read through.
	 */
	private consume(f: (it: Iter<V>) => void, onPanic?: () => void): void {
		const front = this.front;
		this.front = undefined;
		if (front !== undefined) f(front);
		const outer = this.outer;
		this.outer = undefined;
		if (outer !== undefined) {
			consumed(outer, () => {
				for (;;) {
					let inner: T | Done;
					try {
						inner = outer.next();
					} catch (e) {
						onPanic?.();
						throw e;
					}
					if (inner === DONE) return;
					f(this.into(inner));
				}
			});
		}
		const back = this.back;
		this.back = undefined;
		if (back !== undefined) f(back);
	}

	/**
	 * std's: the last of each iterator in turn, each dropped once read, and the last so far
	 * dropped once a later one has one.
	 */
	override lastItem(drop?: Drop<V>): V | Done {
		let last: V | Done = DONE;
		const take = (it: Iter<V>): void => {
			let item: V | Done;
			try {
				item = consumed(it, () => it.lastItem(drop));
			} catch (e) {
				if (last !== DONE) drop?.(last);
				throw e;
			}
			if (item === DONE) return;
			const prev = last;
			last = item;
			// A panic here leaks the later one, as std's unwinding does a returned value.
			if (prev !== DONE) drop?.(prev);
		};
		this.consume(take, () => {
			if (last !== DONE) drop?.(last);
		});
		return last;
	}

	override [Symbol.dispose](): void {
		const outer = this.outer;
		const front = this.front;
		const back = this.back;
		this.outer = undefined;
		this.front = undefined;
		this.back = undefined;
		both(
			() => outer?.[Symbol.dispose](),
			() =>
				both(
					() => front?.[Symbol.dispose](),
					() => back?.[Symbol.dispose](),
				),
		);
	}
}

/** `Peekable`, which drops the item it peeked by `drop`. */
export class Peekable<T> extends Adaptor<T, T> {
	peeked = false;
	item: T | Done = DONE;
	readonly drop: Drop<T> | undefined;

	constructor(inner: Iter<T>, drop?: Drop<T>) {
		super(inner);
		this.drop = drop;
	}

	/** The item it peeked, if any, which it no longer holds. */
	private take(): T | Done {
		this.peeked = false;
		const item = this.item;
		this.item = DONE;
		return item;
	}

	next(): T | Done {
		if (!this.peeked) return this.inner.next();
		return this.take();
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

	/** std's: the item it peeked drops after what its iterator skips. */
	override nthItem(n: number, drop?: Drop<T>): T | Done {
		if (!this.peeked) return this.inner.nthItem(n, drop);
		const peeked = this.take();
		if (peeked === DONE) return DONE;
		if (n === 0) return peeked;
		let item: T | Done;
		try {
			item = this.inner.nthItem(n - 1, drop);
		} catch (e) {
			drop?.(peeked);
			throw e;
		}
		// A panic here leaks the item, as std's unwinding does a returned value.
		drop?.(peeked);
		return item;
	}

	/** std's: the item it peeked drops after its iterator is counted. */
	override countItems(drop?: Drop<T>): number {
		if (!this.peeked) return consumed(this.inner, () => this.inner.countItems(drop));
		const peeked = this.take();
		if (peeked === DONE) return 0;
		try {
			return 1 + consumed(this.inner, () => this.inner.countItems(drop));
		} finally {
			drop?.(peeked);
		}
	}

	/** std's: the item it peeked, unless its iterator has a last, which drops it after. */
	override lastItem(drop?: Drop<T>): T | Done {
		let peeked: T | Done = DONE;
		if (this.peeked) {
			peeked = this.take();
			if (peeked === DONE) return DONE;
		}
		let last: T | Done;
		try {
			last = consumed(this.inner, () => this.inner.lastItem(drop));
		} catch (e) {
			if (peeked !== DONE) drop?.(peeked);
			throw e;
		}
		if (last === DONE) return peeked;
		// A panic here leaks the last, as std's unwinding does a returned value.
		if (peeked !== DONE) drop?.(peeked);
		return last;
	}

	override len(): number {
		if (!this.peeked) return this.inner.len();
		return this.item === DONE ? 0 : this.inner.len() + 1;
	}

	override [Symbol.dispose](): void {
		both(
			() => this.inner[Symbol.dispose](),
			() => {
				const item = this.take();
				if (item !== DONE) this.drop?.(item);
			},
		);
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

/** `Peekable::next_if`: an item `f` rejects is peeked again, and one it panics on drops. */
export function peekableNextIf<T>(p: Peekable<T>, f: (item: T) => boolean): T | undefined {
	const item = p.next();
	if (item !== DONE && (p.drop === undefined ? f(item) : test(f, item, p.drop))) return item;
	p.peeked = true;
	p.item = item;
	return undefined;
}

/** What `iterCollectResult` collects from: values until the first error, which it keeps. */
class ResultShunt<U, E> extends Adaptor<Result<U, E>, U> {
	error: { value: E } | undefined;

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
class OptionShunt<U> extends Adaptor<U | undefined, U> {
	none = false;

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

	next(): T | Done {
		if (rangeInclusiveIsEmpty(this)) return DONE;
		const lo = this.lo;
		if (lt(lo, this.hi)) this.lo = step(lo, 1, this.int)!;
		else this.exhausted = true;
		return lo;
	}

	override nextBack(): T | Done {
		if (rangeInclusiveIsEmpty(this)) return DONE;
		const hi = this.hi;
		if (lt(this.lo, hi)) this.hi = step(hi, -1, this.int)!;
		else this.exhausted = true;
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

/** `Step::forward_overflowing` and `backward_overflowing`: `n + by`, wrapped within `int`, and whether it wrapped. */
function overflowing<T>(n: T, by: number, int: Step<T> | undefined): [T, boolean] {
	if (int === undefined) throw new Error("rust2-ts: stepping a range of no integer type");
	return int(n, by);
}

/** An iterator as itself: what `into_iter` makes of one. */
function itself<T>(iter: Iter<T>): Iter<T> {
	return iter;
}
