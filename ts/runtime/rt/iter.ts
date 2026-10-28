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
// takes the iterator by value, which drops it as std's does, a panic included. Each kind's
// `nth`, `nth_back`, and `advance_by` are its own, as std's are, reading and dropping what
// they skip in its order. A consumer std folds reads what `folded` returns, which steps
// an inclusive range as its `try_fold` does. Where std's `count` and `last` drop items in an
// order each kind of iterator has its own, the `owned` functions follow it, dispatching
// on the kind, so a bundle that never drops an item carries none of it.

import { ByteVec, byteVecPush, Err, type None, Ok, type Place, panic, type RangeBound, type Result } from "../rt.ts";
import { AtIndex, BTreeIter, HashIter } from "./collections.ts";
import { type Limbs, lt as wideLt, type Step } from "./int.ts";
import { drop as dispose, dropAll, dropRange } from "./task.ts";

/** What `next` returns once an iterator is exhausted, which no item is. */
export const DONE: unique symbol = /* @__PURE__ */ Symbol("DONE");

/** The type of `DONE`. */
export type Done = typeof DONE;

/** wasm32's `usize::MAX`, where std's adaptors saturate or split a count. */
const USIZE_MAX = 0xffffffff;

/** What drops a value. */
type Drop<T> = (value: T) => void;

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
	nth(n: number, drop?: Drop<T>): T | Done {
		return this.advanceBy(n, drop) === 0 ? this.next() : DONE;
	}

	/** `nth_back`, as `next_back` returns it. */
	nthBack(n: number, drop?: Drop<T>): T | Done {
		return this.advanceBackBy(n, drop) === 0 ? this.nextBack() : DONE;
	}

	/** What std's `try_fold` reads it by: itself, but for an inclusive range, which steps its own way. */
	folded(): Iter<T> {
		return this;
	}

	/** Its items, for a JS `for ... of`. */
	*[Symbol.iterator](): Iterator<T> {
		for (let item = this.next(); item !== DONE; item = this.next()) yield item;
	}
}

/** An adaptor of `inner`, which drops with it. */
abstract class Adaptor<S, T> extends Iter<T> implements Disposable {
	readonly inner: Iter<S>;

	constructor(inner: Iter<S>) {
		super();
		this.inner = inner;
	}

	[Symbol.dispose](): void {
		dispose(this.inner);
	}
}

/**
 * An adaptor of `inner` that calls `f`, which drops after it, as std's fields do, once:
 * a consumer may drop `inner` before dropping the adaptor that holds it.
 */
abstract class Calling<S, T, F> extends Adaptor<S, T> {
	protected readonly f: F;
	private dropped = false;

	constructor(inner: Iter<S>, f: F) {
		super(inner);
		this.f = f;
	}

	override [Symbol.dispose](): void {
		if (this.dropped) return;
		this.dropped = true;
		both(
			() => dispose(this.inner),
			() => dispose(this.f),
		);
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
export function iterToOption<T>(item: T | Done): T | undefined;
export function iterToOption<T>(item: T | Done, none: None): T | None;
export function iterToOption<T>(item: T | Done, none?: None): T | undefined | None {
	return item === DONE ? none : item;
}

/** An `Option`, as `next` returns it. */
export function iterFromOption<T>(option: T | undefined): T | Done;
export function iterFromOption<T>(option: T | None, none: None): T | Done;
export function iterFromOption<T>(option: T | undefined | None, none?: None): T | Done {
	return option === none ? DONE : (option as T);
}

// Where std's iterators come from.

/** `slice::Iter`, and with what drops each item, what iterating a `Vec` or array by value makes. */
export function iterSlice<T>(s: ArrayLike<T>, drop?: Drop<T>): Iter<T> {
	return new SliceIter(s, drop);
}

/** `slice::IterMut` of elements JS copies: the place of each. */
export function iterSliceMut<T>(s: { [i: number]: T; length: number }): Iter<Place<T>> {
	return new SliceMutIter(s);
}

/** `Option::iter`, and with what drops its item, `option::IntoIter`. */
export function iterOption<T>(o: T | undefined, drop?: Drop<T>): Iter<T>;
export function iterOption<T>(o: T | None, drop: Drop<T> | undefined, none: None): Iter<T>;
export function iterOption<T>(o: T | undefined | None, drop?: Drop<T>, none?: None): Iter<T> {
	return new OptionIter(o as T, o !== none, drop);
}

/** `iter::once`. */
export function iterOnce<T>(value: T, drop?: Drop<T>): Iter<T> {
	return new OptionIter<T>(value, true, drop);
}

/** `iter::empty`. */
export function iterEmpty<T>(): Iter<T> {
	return new OptionIter<T>(undefined, false);
}

/** `iter::from_fn`, whose closure returns an `Option`, and drops with it. */
export function iterFromFn<T>(f: () => T | undefined): Iter<T> {
	return new FromNext(f, (f) => iterFromOption(f()), dispose);
}

/**
 * `iter::successors`: `first`, then `succ` of each item, called as that item is yielded.
 * The item it holds next drops by `drop`, and then `succ`, as std's fields drop.
 */
export function iterSuccessors<T>(first: T | undefined, succ: (item: T) => T | undefined, drop?: Drop<T>): Iter<T> {
	return new SuccessorsIter(first, succ, drop);
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
	const folded = items.folded();
	if (dropIt === undefined) {
		for (let item = folded.next(); item !== DONE; item = folded.next()) v.push(item);
		return;
	}
	try {
		for (let item = folded.next(); item !== DONE; item = folded.next()) v.push(item);
	} finally {
		dropIt(items);
	}
}

/** `Extend::extend` of a `Vec<u8>`. */
export function iterExtendBytes(v: ByteVec, items: Iter<number>, dropIt?: Drop<Iter<number>>): void {
	const folded = items.folded();
	try {
		for (let item = folded.next(); item !== DONE; item = folded.next()) byteVecPush(v, item);
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

/** `Iterator::skip`, dropping what it skips by `drop`, through its iterator's own `nth`. */
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

/**
 * `Iterator::copied` of items JS copies by `copy`, which std's skips through: its `nth`
 * and `advance_by` are its iterator's.
 */
export function iterCopied<R, T>(it: Iter<R>, copy: (item: R) => T): Iter<T> {
	return new CopiedIter(it, copy);
}

/** `Iterator::cloned` of items that are JS values, whose `nth` reads by `next`, as std's does. */
export function iterCloned<T>(it: Iter<T>): Iter<T> {
	return new MapIter(it, same);
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

/** `Iterator::count` of an iterator that drops nothing. */
export function iterCount<T>(it: Iter<T>): number {
	const folded = it.folded();
	let n = 0;
	while (folded.next() !== DONE) n++;
	return n;
}

/** `Iterator::last` of an iterator that drops nothing. */
export function iterLast<T>(it: Iter<T>): T | undefined {
	const folded = it.folded();
	let last: T | Done = DONE;
	for (let item = folded.next(); item !== DONE; item = folded.next()) last = item;
	return iterToOption(last);
}

/** `Iterator::nth`, dropping what it skips by `drop`, as its own `nth` does. */
export function iterNth<T>(it: Iter<T>, n: number, drop?: Drop<T>): T | undefined {
	return iterToOption(it.nth(n, drop));
}

/** `DoubleEndedIterator::nth_back`, dropping what it skips by `drop`, as its own `nth_back` does. */
export function iterNthBack<T>(it: Iter<T>, n: number, drop?: Drop<T>): T | undefined;
export function iterNthBack<T>(it: Iter<T>, n: number, drop: Drop<T> | undefined, none: None): T | None;
export function iterNthBack<T>(it: Iter<T>, n: number, drop?: Drop<T>, none?: None): T | undefined | None {
	const item = it.nthBack(n, drop);
	return item === DONE ? none : item;
}

/** `Iterator::any`. */
export function iterAny<T>(it: Iter<T>, f: (item: T) => boolean): boolean {
	const folded = it.folded();
	for (let item = folded.next(); item !== DONE; item = folded.next()) if (f(item)) return true;
	return false;
}

/** `Iterator::all`. */
export function iterAll<T>(it: Iter<T>, f: (item: T) => boolean): boolean {
	const folded = it.folded();
	for (let item = folded.next(); item !== DONE; item = folded.next()) if (!f(item)) return false;
	return true;
}

/** `find`, as `next` returns it: what `f` rejects, or panics on, drops by `drop`. */
function findItem<T>(it: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>): T | Done {
	const folded = it.folded();
	if (drop === undefined) {
		for (let item = folded.next(); item !== DONE; item = folded.next()) if (f(item)) return item;
		return DONE;
	}
	for (let item = folded.next(); item !== DONE; item = folded.next()) {
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
function test<T, R>(f: (item: T) => R, item: T, drop: Drop<T>): R {
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
	const folded = it.folded();
	for (let item = folded.next(); item !== DONE; item = folded.next()) {
		const found = f(item);
		if (found !== undefined) return found;
	}
	return undefined;
}

/** `Iterator::position`. */
export function iterPosition<T>(it: Iter<T>, f: (item: T) => boolean): number | undefined {
	const folded = it.folded();
	let i = 0;
	for (let item = folded.next(); item !== DONE; item = folded.next()) {
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
	const folded = it.folded();
	let acc = init;
	if (dropIt === undefined && dropAcc === undefined) {
		for (let item = folded.next(); item !== DONE; item = folded.next()) acc = f(acc, item);
		return acc;
	}
	try {
		for (;;) {
			let item: T | Done;
			try {
				item = folded.next();
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
	const folded = it.folded();
	if (dropIt === undefined) {
		for (let item = folded.next(); item !== DONE; item = folded.next()) f(item);
		return;
	}
	try {
		for (let item = folded.next(); item !== DONE; item = folded.next()) f(item);
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
	const folded = it.folded();
	if (dropIt === undefined && drop === undefined) {
		let best = it.next();
		if (best === DONE) return undefined;
		let key = f(best);
		for (let item = folded.next(); item !== DONE; item = folded.next()) {
			const k = f(item);
			if (least ? lt(k, key) : !lt(k, key)) [best, key] = [item, k];
		}
		return best;
	}
	let best: T | Done = DONE;
	let key: K | undefined;
	try {
		for (let item = it.next(); item !== DONE; item = folded.next()) {
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
	const folded = it.folded();
	const out: T[] = [];
	if (dropIt === undefined && drop === undefined) {
		for (let item = folded.next(); item !== DONE; item = folded.next()) out.push(item);
		return out;
	}
	// Not `dropAll`'s default, which would drop what a collected reference points to.
	const collected = () => drop !== undefined && dropAll(out, drop);
	try {
		for (let item = folded.next(); item !== DONE; item = folded.next()) out.push(item);
	} catch (e) {
		if (inPlace) both(collected, () => dropIt?.(it));
		else both(() => dropIt?.(it), collected);
		throw e;
	}
	try {
		dropIt?.(it);
	} catch (e) {
		collected();
		throw e;
	}
	return out;
}

/** `collect` into a `Vec<u8>`. */
export function iterToByteVec(it: Iter<number>, dropIt?: Drop<Iter<number>>): ByteVec {
	const folded = it.folded();
	const out = new ByteVec();
	// Its own loop: through `iterExtendBytes`, JavaScriptCore took 1.9x as long.
	if (dropIt === undefined) {
		for (let item = folded.next(); item !== DONE; item = folded.next()) byteVecPush(out, item);
		return out;
	}
	try {
		for (let item = folded.next(); item !== DONE; item = folded.next()) byteVecPush(out, item);
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

// What std's iterators override to count or take the last: `count` and `last`, each of
// which drops items in an order of its own kind's. These follow it, dispatching on the
// kind, rather than as methods every iterator would carry, and the translation calls them
// only where an item or the iterator drops.

/** `Iterator::count` of an iterator that drops what it holds: each item by `drop`, as its own `count` does, and then it by `dropIt`, a panic included. */
export function ownedCount<T>(it: Iter<T>, dropIt?: Drop<Iter<T>>, drop?: Drop<T>): number {
	try {
		return count(it, drop);
	} finally {
		dropIt?.(it);
	}
}

/** `Iterator::last` of an iterator that drops what it holds: each item but the last by `drop`, as its own `last` does, and then it by `dropIt`. */
export function ownedLast<T>(it: Iter<T>, dropIt?: Drop<Iter<T>>, drop?: Drop<T>): T | undefined {
	let item: T | Done;
	try {
		item = last(it, drop);
	} catch (e) {
		dropIt?.(it);
		throw e;
	}
	// A panic here leaks the last, as std's unwinding does a returned value.
	dropIt?.(it);
	return iterToOption(item);
}

/** `count`, dropping each item by `drop`. */
function count<T>(it: Iter<T>, drop: Drop<T> | undefined): number {
	if (it instanceof EnumerateIter) return count(it.inner, items(drop));
	if (it instanceof ChainIter) {
		// std's: each iterator's own `count`, the first's first.
		const a = it.a;
		it.a = undefined;
		const n = a === undefined ? 0 : consumed(a, () => count(a, drop));
		const b = it.b;
		it.b = undefined;
		return n + (b === undefined ? 0 : consumed(b, () => count(b, drop)));
	}
	if (it instanceof SkipIter) {
		// std's: past what it skips by `nth`, and then its iterator's own `count`.
		if (skipped(it, drop)) return 0;
		return consumed(it.inner, () => count(it.inner, drop));
	}
	if (it instanceof TakeIter) {
		// std's: its iterator advanced through the limit, dropping what it passes.
		if (it.n === 0) return 0;
		const n = it.n;
		it.n = 0;
		return n - it.inner.advanceBy(n, drop);
	}
	if (it instanceof FuseIter) {
		const inner = it.inner;
		it.inner = undefined;
		return inner === undefined ? 0 : consumed(inner, () => count(inner, drop));
	}
	if (it instanceof FlattenIter) {
		// std's: each iterator's own `count` in turn, each dropped once counted.
		let n = 0;
		flattenEach(it, (inner) => {
			n += consumed(inner, () => count(inner, drop));
		});
		return n;
	}
	if (it instanceof Peekable) {
		// std's: the item it peeked drops after its iterator is counted.
		if (!it.peeked) return consumed(it.inner, () => count(it.inner, drop));
		const peeked = it.take();
		if (peeked === DONE) return 0;
		try {
			return 1 + consumed(it.inner, () => count(it.inner, drop));
		} finally {
			drop?.(peeked);
		}
	}
	if (it instanceof HashIter && !it.drain) {
		// std's `into_iter`'s: its length, and then each entry drops with it.
		const n = it.left;
		dispose(it);
		return n;
	}
	const folded = it.folded();
	let n = 0;
	for (let item = folded.next(); item !== DONE; item = folded.next()) {
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
function last<T>(it: Iter<T>, drop: Drop<T> | undefined): T | Done {
	if ((it instanceof SliceIter && drop !== undefined) || (it instanceof BTreeIter && it.back)) {
		// std's, of a `Vec`'s or an array's, and of a B-tree's `into_keys` and
		// `into_values`: the last item, and then the rest drop with it.
		const item = it.nextBack();
		// A panic here leaks the last, as std's unwinding does a returned value.
		dispose(it);
		return item;
	}
	if (it instanceof ChainIter) {
		// std's: each iterator's last, the first's held while the second is read, and
		// dropped once the second has one.
		const a = it.a;
		it.a = undefined;
		const aLast = a === undefined ? DONE : consumed(a, () => last(a, drop));
		const b = it.b;
		it.b = undefined;
		if (b === undefined) return aLast;
		let bLast: T | Done;
		try {
			bLast = consumed(b, () => last(b, drop));
		} catch (e) {
			if (aLast !== DONE) drop?.(aLast);
			throw e;
		}
		if (bLast === DONE) return aLast;
		// A panic here leaks the second's, as std's unwinding does a returned value.
		if (aLast !== DONE) drop?.(aLast);
		return bLast;
	}
	if (it instanceof SkipIter) {
		// std's: past what it skips by `nth`, and then its iterator's own `last`.
		if (skipped(it, drop)) return DONE;
		return consumed(it.inner, () => last(it.inner, drop));
	}
	if (it instanceof FuseIter) {
		const inner = it.inner;
		it.inner = undefined;
		return inner === undefined ? DONE : consumed(inner, () => last(inner, drop));
	}
	if (it instanceof FlattenIter) return flattenLast(it, drop);
	if (it instanceof Peekable) {
		// std's: the item it peeked, unless its iterator has a last, which drops it after.
		let peeked: T | Done = DONE;
		if (it.peeked) {
			peeked = it.take();
			if (peeked === DONE) return DONE;
		}
		let item: T | Done;
		try {
			item = consumed(it.inner, () => last(it.inner, drop));
		} catch (e) {
			if (peeked !== DONE) drop?.(peeked);
			throw e;
		}
		if (item === DONE) return peeked;
		// A panic here leaks the last, as std's unwinding does a returned value.
		if (peeked !== DONE) drop?.(peeked);
		return item;
	}
	const folded = it.folded();
	let item: T | Done = DONE;
	if (drop === undefined) {
		for (let next = folded.next(); next !== DONE; next = folded.next()) item = next;
		return item;
	}
	for (;;) {
		let next: T | Done;
		try {
			next = folded.next();
		} catch (e) {
			if (item !== DONE) drop(item);
			throw e;
		}
		if (next === DONE) return item;
		const prev = item;
		item = next;
		if (prev !== DONE) drop(prev);
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
		dispose(it);
		throw e;
	}
	dispose(it);
	return r;
}

/** What drops an `enumerate`'s item, of one its iterator yields: `drop` is of `[number, T]`. */
function items(drop: Drop<never> | undefined): Drop<unknown> | undefined {
	return drop === undefined ? undefined : (item) => drop([0, item] as never);
}

/** Read past what `it` still skips, as `nth` reads, and whether its iterator ended first. */
function skipped<T>(it: SkipIter<T>, drop: Drop<T> | undefined): boolean {
	if (it.n === 0) return false;
	const n = it.n;
	it.n = 0;
	const item = it.inner.nth(n - 1, drop);
	if (item === DONE) return true;
	drop?.(item);
	return false;
}

/**
 * Run `f` on each iterator of `it` in turn, the front, each of the source's, and the back,
 * as std's `fold` takes them by value; the source drops once read through.
 */
function flattenEach<T, V>(it: FlattenIter<T, V>, f: (inner: Iter<V>) => void, onPanic?: () => void): void {
	const front = it.front;
	it.front = undefined;
	if (front !== undefined) f(front);
	const outer = it.outer;
	it.outer = undefined;
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
				f(it.into(inner));
			}
		});
	}
	const back = it.back;
	it.back = undefined;
	if (back !== undefined) f(back);
}

/**
 * `Flatten::last`, std's: the last of each iterator in turn, each dropped once read, and
 * the last so far dropped once a later one has one.
 */
function flattenLast<T, V>(it: FlattenIter<T, V>, drop: Drop<V> | undefined): V | Done {
	let item: V | Done = DONE;
	const take = (inner: Iter<V>): void => {
		let next: V | Done;
		try {
			next = consumed(inner, () => last(inner, drop));
		} catch (e) {
			if (item !== DONE) drop?.(item);
			throw e;
		}
		if (next === DONE) return;
		const prev = item;
		item = next;
		// A panic here leaks the later one, as std's unwinding does a returned value.
		if (prev !== DONE) drop?.(prev);
	};
	flattenEach(it, take, () => {
		if (item !== DONE) drop?.(item);
	});
	return item;
}

// Iterators. One has `[Symbol.dispose]` if it owns items or wraps an iterator: what holds
// it drops it by `rt.drop`, not knowing its kind.

class SliceIter<T> extends Iter<T> implements Disposable {
	readonly s: ArrayLike<T>;
	front = 0;
	back: number;
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

	[Symbol.dispose](): void {
		if (this.drop === undefined) return;
		const from = this.front;
		this.front = this.back;
		dropRange(this.s, this.drop, from, this.back);
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
}

/** The places of a slice's elements, from both ends. */
class SliceMutIter<T> extends Iter<Place<T>> {
	private readonly s: { [i: number]: T; length: number };
	private front = 0;
	private back: number;

	constructor(s: { [i: number]: T; length: number }) {
		super();
		this.s = s;
		this.back = s.length;
	}

	next(): Place<T> | Done {
		return this.front < this.back ? new AtIndex(this.s, this.front++) : DONE;
	}

	override nextBack(): Place<T> | Done {
		return this.front < this.back ? new AtIndex(this.s, --this.back) : DONE;
	}

	override len(): number {
		return this.back - this.front;
	}

	override advanceBy(n: number): number {
		const step = Math.min(n, this.back - this.front);
		this.front += step;
		return n - step;
	}

	override advanceBackBy(n: number): number {
		const step = Math.min(n, this.back - this.front);
		this.back -= step;
		return n - step;
	}
}

/** `option::IntoIter`, `Once`, and `Empty`: at most one item. */
class OptionIter<T> extends Iter<T> implements Disposable {
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

	[Symbol.dispose](): void {
		if (this.drop === undefined) return;
		const item = this.next();
		if (item !== DONE) this.drop(item);
	}
}

/** An iterator whose `next` is a function of its source, which `drop` drops. */
class FromNext<S, T> extends Iter<T> implements Disposable {
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

	[Symbol.dispose](): void {
		const drop = this.drop;
		this.drop = undefined;
		drop?.(this.source);
	}
}

/** `Successors`, holding the item it yields next, which drops once, as `Calling` does. */
class SuccessorsIter<T> extends Iter<T> implements Disposable {
	private item: T | undefined;
	private readonly succ: (item: T) => T | undefined;
	private readonly drop: Drop<T> | undefined;
	private dropped = false;

	constructor(first: T | undefined, succ: (item: T) => T | undefined, drop?: Drop<T>) {
		super();
		this.item = first;
		this.succ = succ;
		this.drop = drop;
	}

	next(): T | Done {
		const item = this.item;
		if (item === undefined) return DONE;
		// Taken before `succ` runs, so a panic in it leaves the iterator done, as std's.
		this.item = undefined;
		if (this.drop === undefined) this.item = this.succ(item);
		else this.item = test(this.succ, item, this.drop);
		return item;
	}

	[Symbol.dispose](): void {
		if (this.dropped) return;
		this.dropped = true;
		const item = this.item;
		this.item = undefined;
		both(
			() => {
				if (item !== undefined) this.drop?.(item);
			},
			() => dispose(this.succ),
		);
	}
}

/**
 * An iterator borrowed by `&mut`, which forwards only what std's `impl Iterator for &mut
 * I` does, and which dropping drops nothing.
 */
class Borrowed<T> extends Iter<T> {
	readonly inner: Iter<T>;

	constructor(inner: Iter<T>) {
		super();
		this.inner = inner;
	}

	next(): T | Done {
		return this.inner.next();
	}

	override folded(): Iter<T> {
		return this.inner.folded();
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

	override nth(n: number, drop?: Drop<T>): T | Done {
		return this.inner.nth(n, drop);
	}

	override nthBack(n: number, drop?: Drop<T>): T | Done {
		return this.inner.nthBack(n, drop);
	}
}

class MapIter<T, U> extends Calling<T, U, (item: T) => U> {

	constructor(inner: Iter<T>, f: (item: T) => U) {
		super(inner, f);
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

/** `Copied` of items JS copies by a function: a `map` whose `nth` and `advance_by` are its iterator's. */
class CopiedIter<R, T> extends MapIter<R, T> {
	override advanceBy(n: number): number {
		return this.inner.advanceBy(n);
	}

	override advanceBackBy(n: number): number {
		return this.inner.advanceBackBy(n);
	}

	override nth(n: number): T | Done {
		const item = this.inner.nth(n);
		return item === DONE ? DONE : this.f(item);
	}
}

class FilterIter<T> extends Calling<T, T, (item: T) => boolean> {
	private readonly drop: Drop<T> | undefined;

	constructor(inner: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>) {
		super(inner, f);
		this.drop = drop;
	}

	next(): T | Done {
		return findItem(this.inner, this.f, this.drop);
	}

	override nextBack(): T | Done {
		return rfindItem(this.inner, this.f, this.drop);
	}
}

class FilterMapIter<T, U> extends Calling<T, U, (item: T) => U | undefined> {

	constructor(inner: Iter<T>, f: (item: T) => U | undefined) {
		super(inner, f);
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
	index = 0;

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

	/** std's, which counts what its iterator skipped. */
	override advanceBy(n: number, drop?: Drop<[number, T]>): number {
		const left = this.inner.advanceBy(n, items(drop));
		this.index += n - left;
		return left;
	}

	override advanceBackBy(n: number, drop?: Drop<[number, T]>): number {
		return this.inner.advanceBackBy(n, items(drop));
	}

	/** std's, which counts only when it finds an item. */
	override nth(n: number, drop?: Drop<[number, T]>): [number, T] | Done {
		const item = this.inner.nth(n, items(drop));
		if (item === DONE) return DONE;
		const i = this.index + n;
		this.index = i + 1;
		return [i, item];
	}

	override nthBack(n: number, drop?: Drop<[number, T]>): [number, T] | Done {
		const item = this.inner.nthBack(n, items(drop));
		return item === DONE ? DONE : [this.index + this.inner.len(), item];
	}
}

/**
 * `Chain`, which drops its first iterator once it ends, but not its second, as std's
 * stops reading it.
 */
class ChainIter<T> extends Iter<T> implements Disposable {
	a: Iter<T> | undefined;
	b: Iter<T> | undefined;

	constructor(a: Iter<T>, b: Iter<T>) {
		super();
		this.a = a;
		this.b = b;
	}

	/** Drop `a`, which ended. */
	endA(): void {
		const a = this.a;
		this.a = undefined;
		dispose(a);
	}

	/** Drop `b`, which ended from the back. */
	endB(): void {
		const b = this.b;
		this.b = undefined;
		dispose(b);
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

	[Symbol.dispose](): void {
		both(
			() => this.endA(),
			() => this.endB(),
		);
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

	override nth(n: number, drop?: Drop<T>): T | Done {
		if (this.a !== undefined) {
			n = this.a.advanceBy(n, drop);
			if (n === 0) {
				const item = this.a.next();
				if (item !== DONE) return item;
			}
			this.endA();
		}
		return this.b === undefined ? DONE : this.b.nth(n, drop);
	}

	override nthBack(n: number, drop?: Drop<T>): T | Done {
		if (this.b !== undefined) {
			n = this.b.advanceBackBy(n, drop);
			if (n === 0) {
				const item = this.b.nextBack();
				if (item !== DONE) return item;
			}
			this.endB();
		}
		return this.a === undefined ? DONE : this.a.nthBack(n, drop);
	}
}

class ZipIter<A, B> extends Iter<[A, B]> implements Disposable {
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

	[Symbol.dispose](): void {
		both(
			() => dispose(this.a),
			() => dispose(this.b),
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

	override len(): number {
		return this.inner.len();
	}

	override advanceBy(n: number, drop?: Drop<T>): number {
		return this.inner.advanceBackBy(n, drop);
	}

	override advanceBackBy(n: number, drop?: Drop<T>): number {
		return this.inner.advanceBy(n, drop);
	}

	override nth(n: number, drop?: Drop<T>): T | Done {
		return this.inner.nthBack(n, drop);
	}

	override nthBack(n: number, drop?: Drop<T>): T | Done {
		return this.inner.nth(n, drop);
	}
}

/** `Skip`, whose items drop by `drop` as its iterator's own `nth` skips them. */
class SkipIter<T> extends Adaptor<T, T> {
	n: number;
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
		return this.inner.nth(n, this.drop);
	}

	override nextBack(): T | Done {
		return this.len() > 0 ? this.inner.nextBack() : DONE;
	}

	override len(): number {
		return Math.max(this.inner.len() - this.n, 0);
	}

	/** std's: what it skips and `n` more in one step, and the rest of `n` if that saturated a `usize`. */
	override advanceBy(n: number, drop?: Drop<T>): number {
		const skip = this.n;
		const both = Math.min(skip + n, USIZE_MAX);
		const left = this.inner.advanceBy(both, drop);
		const advanced = both - left;
		n -= Math.max(advanced - skip, 0);
		this.n = Math.max(skip - advanced, 0);
		if (left === 0 && n > 0) n = this.inner.advanceBy(n, drop);
		return n;
	}

	/** std's: its iterator advanced from the back in one step, short of what it skips. */
	override advanceBackBy(n: number, drop?: Drop<T>): number {
		const min = Math.min(this.len(), n);
		this.inner.advanceBackBy(min, drop);
		return n - min;
	}

	/**
	 * std's: what it still skips and then `n` more, in one step, or past a `usize`'s
	 * greatest, what it skips in one and `n` in another.
	 */
	override nth(n: number, drop?: Drop<T>): T | Done {
		const skip = this.n;
		this.n = 0;
		if (skip + n <= USIZE_MAX) return this.inner.nth(skip + n, drop);
		const item = this.inner.nth(skip - 1, drop);
		if (item === DONE) return DONE;
		drop?.(item);
		return this.inner.nth(n, drop);
	}

	/** std's: past what it doesn't skip, its iterator read from the back through what it does. */
	override nthBack(n: number, drop?: Drop<T>): T | Done {
		const len = this.len();
		if (n < len) return this.inner.nthBack(n, drop);
		if (len > 0) {
			const item = this.inner.nthBack(len - 1, drop);
			if (item !== DONE) drop?.(item);
		}
		return DONE;
	}
}

/** `Take`, of which what its iterator holds past the limit drops by `drop` as `next_back` reads through it. */
class TakeIter<T> extends Adaptor<T, T> {
	n: number;
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

	/** std's: what its iterator holds past the limit is read through from the back. */
	override nextBack(): T | Done {
		if (this.n === 0) return DONE;
		const n = this.n--;
		return this.inner.nthBack(Math.max(this.inner.len() - n, 0), this.drop);
	}

	override len(): number {
		return Math.min(this.inner.len(), this.n);
	}

	override advanceBy(n: number, drop?: Drop<T>): number {
		const min = Math.min(this.n, n);
		const advanced = min - this.inner.advanceBy(min, drop);
		this.n -= advanced;
		return n - advanced;
	}

	/** std's: its iterator advanced from the back past the limit and `n` more, in one step. */
	override advanceBackBy(n: number, drop?: Drop<T>): number {
		const trim = Math.max(this.inner.len() - this.n, 0);
		const advanced = trim + n - this.inner.advanceBackBy(trim + n, drop) - trim;
		this.n -= advanced;
		return n - advanced;
	}

	/** std's: past the limit, it reads the rest of the limit through, drops it, and ends. */
	override nth(n: number, drop?: Drop<T>): T | Done {
		if (this.n > n) {
			this.n -= n + 1;
			return this.inner.nth(n, drop);
		}
		if (this.n > 0) {
			const item = this.inner.nth(this.n - 1, drop);
			if (item !== DONE) drop?.(item);
			this.n = 0;
		}
		return DONE;
	}

	/** std's: past the limit, it reads the whole rest from the back, even at a limit of 0. */
	override nthBack(n: number, drop?: Drop<T>): T | Done {
		const len = this.inner.len();
		if (this.n > n) {
			const m = Math.max(len - this.n, 0) + n;
			this.n -= n + 1;
			return this.inner.nthBack(m, drop);
		}
		if (len > 0) {
			const item = this.inner.nthBack(len - 1, drop);
			if (item !== DONE) drop?.(item);
		}
		return DONE;
	}
}

class SkipWhileIter<T> extends Calling<T, T, (item: T) => boolean> {
	private readonly drop: Drop<T> | undefined;
	private flag = false;

	constructor(inner: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>) {
		super(inner, f);
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

class TakeWhileIter<T> extends Calling<T, T, (item: T) => boolean> {
	private readonly drop: Drop<T> | undefined;
	private flag = false;

	constructor(inner: Iter<T>, f: (item: T) => boolean, drop?: Drop<T>) {
		super(inner, f);
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
class MapWhileIter<T, U> extends Calling<T, U, (item: T) => U | undefined> {

	constructor(inner: Iter<T>, f: (item: T) => U | undefined) {
		super(inner, f);
	}

	next(): U | Done {
		const item = this.inner.next();
		return item === DONE ? DONE : iterFromOption(this.f(item));
	}
}

/** `StepBy`, whose items drop by `drop` as its iterator's own `nth` and `nth_back` step over them. */
class StepByIter<T> extends Adaptor<T, T> {
	readonly step: number;
	firstTake = true;
	private readonly drop: Drop<T> | undefined;

	constructor(inner: Iter<T>, step: number, drop?: Drop<T>) {
		super(inner);
		// The iterator it was given drops as the panic unwinds.
		if (step === 0) {
			dispose(inner);
			panic("assertion failed: step != 0");
		}
		this.step = step - 1;
		this.drop = drop;
	}

	next(): T | Done {
		const n = this.firstTake ? 0 : this.step;
		this.firstTake = false;
		return this.inner.nth(n, this.drop);
	}

	override nextBack(): T | Done {
		return this.inner.nthBack(this.backIndex(), this.drop);
	}

	/** How far from the back its last item is. */
	backIndex(): number {
		const rem = this.inner.len() % (this.step + 1);
		return this.firstTake ? (rem === 0 ? this.step : rem - 1) : rem;
	}

	override len(): number {
		const n = this.inner.len();
		if (this.firstTake) return n === 0 ? 0 : 1 + Math.floor((n - 1) / (this.step + 1));
		return Math.floor(n / (this.step + 1));
	}

	/**
	 * std's: the first item, dropped unless it is the one, then every step in one `nth`,
	 * as many as a `usize` counts at a time.
	 */
	override nth(n: number, drop?: Drop<T>): T | Done {
		if (this.firstTake) {
			this.firstTake = false;
			const first = this.inner.next();
			if (first === DONE || n === 0) return first;
			drop?.(first);
			n -= 1;
		}
		let step = this.step + 1;
		if (n === USIZE_MAX) {
			const item = this.inner.nth(step - 1, drop);
			if (item === DONE) return DONE;
			drop?.(item);
		} else {
			n += 1;
		}
		for (;;) {
			if (n * step <= USIZE_MAX) return this.inner.nth(n * step - 1, drop);
			const byN = Math.floor(USIZE_MAX / n) * n;
			const byStep = Math.floor(USIZE_MAX / step) * step;
			if (byN > byStep) step -= Math.floor(USIZE_MAX / n);
			else n -= Math.floor(USIZE_MAX / step);
			const item = this.inner.nth(Math.max(byN, byStep) - 1, drop);
			if (item === DONE) return DONE;
			drop?.(item);
		}
	}

	/** std's: every step from the back in one, saturating at a `usize`'s greatest. */
	override nthBack(n: number, drop?: Drop<T>): T | Done {
		const steps = Math.min(n * (this.step + 1), USIZE_MAX);
		return this.inner.nthBack(Math.min(steps + this.backIndex(), USIZE_MAX), drop);
	}
}

class InspectIter<T> extends Calling<T, T, (item: T) => void> {
	private readonly drop: Drop<T> | undefined;

	constructor(inner: Iter<T>, f: (item: T) => void, drop?: Drop<T>) {
		super(inner, f);
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

class FuseIter<T> extends Iter<T> implements Disposable {
	inner: Iter<T> | undefined;
	/** Whether its iterator is a `FusedIterator`, which std's keeps once it ends. */
	private readonly fused: boolean;

	constructor(inner: Iter<T>, fused: boolean) {
		super();
		this.inner = inner;
		this.fused = fused;
	}

	/** `item`, dropping its iterator first if it ended there and is not fused. */
	clear(item: T | Done): T | Done {
		if (item === DONE && !this.fused) {
			const inner = this.inner;
			this.inner = undefined;
			dispose(inner);
		}
		return item;
	}

	next(): T | Done {
		return this.inner === undefined ? DONE : this.clear(this.inner.next());
	}

	override nextBack(): T | Done {
		return this.inner === undefined ? DONE : this.clear(this.inner.nextBack());
	}

	override len(): number {
		return this.inner === undefined ? 0 : this.inner.len();
	}

	[Symbol.dispose](): void {
		const inner = this.inner;
		this.inner = undefined;
		dispose(inner);
	}

	override nth(n: number, drop?: Drop<T>): T | Done {
		return this.inner === undefined ? DONE : this.clear(this.inner.nth(n, drop));
	}

	override nthBack(n: number, drop?: Drop<T>): T | Done {
		return this.inner === undefined ? DONE : this.clear(this.inner.nthBack(n, drop));
	}
}

/**
 * `Flatten` and `FlatMap`: the iterator of each item it reads, which drops once it ends,
 * as does its source unless it is `fused`.
 */
class FlattenIter<T, V> extends Iter<V> implements Disposable {
	outer: Iter<T> | undefined;
	readonly into: (value: T) => Iter<V>;
	readonly fused: boolean;
	front: Iter<V> | undefined;
	back: Iter<V> | undefined;

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
			dispose(outer);
		}
		return item;
	}

	/** `it`'s next item, from the back if `back`, dropping it if it ended. */
	private static step<V>(it: Iter<V>, back: boolean): V | Done {
		const item = back ? it.nextBack() : it.next();
		if (item === DONE) dispose(it);
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
	setFront(it: Iter<V> | undefined): void {
		const old = this.front;
		this.front = it;
		dispose(old);
	}

	setBack(it: Iter<V> | undefined): void {
		const old = this.back;
		this.back = it;
		dispose(old);
	}

	[Symbol.dispose](): void {
		const outer = this.outer;
		const front = this.front;
		const back = this.back;
		this.outer = undefined;
		this.front = undefined;
		this.back = undefined;
		both(
			() => dispose(outer),
			() =>
				both(
					() => dispose(front),
					() => dispose(back),
				),
		);
	}

	/**
	 * std's, by `try_fold`: each iterator in turn advanced by its own `advance_by`, one
	 * that falls short dropped only as the next replaces it, and the source dropped once
	 * it ends unless it is fused.
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
				const front = this.into(inner);
				this.setFront(front);
				n = front.advanceBy(n, drop);
				if (n === 0) return 0;
			}
			if (!this.fused) {
				this.outer = undefined;
				dispose(outer);
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
				const back = this.into(inner);
				this.setBack(back);
				n = back.advanceBackBy(n, drop);
				if (n === 0) return 0;
			}
			if (!this.fused) {
				this.outer = undefined;
				dispose(outer);
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
	take(): T | Done {
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

	override len(): number {
		if (!this.peeked) return this.inner.len();
		return this.item === DONE ? 0 : this.inner.len() + 1;
	}

	override [Symbol.dispose](): void {
		both(
			() => dispose(this.inner),
			() => {
				const item = this.take();
				if (item !== DONE) this.drop?.(item);
			},
		);
	}

	/** std's: the item it peeked drops after what its iterator skips. */
	override nth(n: number, drop?: Drop<T>): T | Done {
		if (!this.peeked) return this.inner.nth(n, drop);
		const peeked = this.take();
		if (peeked === DONE) return DONE;
		if (n === 0) return peeked;
		let item: T | Done;
		try {
			item = this.inner.nth(n - 1, drop);
		} catch (e) {
			drop?.(peeked);
			throw e;
		}
		// A panic here leaks the item, as std's unwinding does a returned value.
		drop?.(peeked);
		return item;
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
	override nth(n: number): T | Done {
		const item = step(this.start, n, this.int);
		if (item !== undefined && lt(item, this.end)) {
			this.start = step(item, 1, this.int)!;
			return item;
		}
		this.start = this.end;
		return DONE;
	}

	/** std's `nth_back`, in one step. */
	override nthBack(n: number): T | Done {
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

	override folded(): Iter<T> {
		return new RangeInclusiveFold(this);
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
	override nth(n: number): T | Done {
		if (rangeInclusiveIsEmpty(this)) return DONE;
		const [item, over] = overflowing(this.lo, n, this.int);
		const [after, overAfter] = overflowing(item, 1, this.int);
		this.lo = after;
		this.exhausted = over || overAfter;
		return !over && !lt(this.hi, item) ? item : DONE;
	}

	/** std's `nth_back`, from the end. */
	override nthBack(n: number): T | Done {
		if (rangeInclusiveIsEmpty(this)) return DONE;
		const [item, over] = overflowing(this.hi, -n, this.int);
		const [before, overBefore] = overflowing(item, -1, this.int);
		this.hi = before;
		this.exhausted = over || overBefore;
		return !over && !lt(item, this.lo) ? item : DONE;
	}
}

/** An inclusive range as std's `try_fold` steps it: `start` stops at `end`, which exhausts it, so it never wraps. */
class RangeInclusiveFold<T> extends Iter<T> {
	readonly range: RangeInclusive<T>;

	constructor(range: RangeInclusive<T>) {
		super();
		this.range = range;
	}

	next(): T | Done {
		const r = this.range;
		if (rangeInclusiveIsEmpty(r)) return DONE;
		const lo = r.lo;
		if (lt(lo, r.hi)) r.lo = step(lo, 1, r.int)!;
		else r.exhausted = true;
		return lo;
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
export function rangeInclusiveStartBound<T>(r: RangeInclusive<T>): RangeBound<T> {
	return { tag: "Included", _0: r.lo };
}

/** `RangeInclusive::end_bound`: its end, included, until wrapping exhausts it and it excludes its start, as std's. */
export function rangeInclusiveEndBound<T>(r: RangeInclusive<T>): RangeBound<T> {
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

/** A value as itself: what `cloned` makes of a JS value. */
function same<T>(value: T): T {
	return value;
}
