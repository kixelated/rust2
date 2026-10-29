// Collections, for rust2-ts output: std's `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet`,
// and `VecDeque`, with `Ordering` and `DefaultHasher`, which their keys need. `rt.ts`
// re-exports all of it. Each collection is one type whose functions are free functions
// here, so a bundler keeps only those a caller reaches.
//
// A key compares as Rust's `Eq` or `Ord` says, never by JS identity: a hash collection
// finds a key that is a JS object through its translated `Hash` and `Eq`, and a B-tree
// orders every key by its translated `Ord`. Which of those calls the runtime makes, and
// how often, are its own, not std's.
//
// No key, value, or item is ever `undefined`, which the translator refuses, so
// `undefined` is a missing one.
//
// A function that may drop a key, value, or item std's would, rather than return or keep
// it, takes what drops each that does, as `k`, `v`, or `f`, and drops it where std's does.

import { type None, type Place, panic } from "../rt.ts";
import { type Limbs, lt } from "./int.ts";
import { DONE, type Done, Iter, iterMap, iterSlice, type RangeInclusive, type Take } from "./iter.ts";
import { type Arc, drop as dispose, dropAll } from "./task.ts";

// Ordering

/** `core::cmp::Ordering`: its discriminant, as `as i8` reads it. */
export type Ordering = -1 | 0 | 1;

/** `Ord::cmp` of two integers of one type, or of two `bool`s. */
export function cmp<T extends number | boolean | Limbs>(a: T, b: T): Ordering {
	if (typeof a !== "object" && typeof b !== "object") return a < b ? -1 : a > b ? 1 : 0;
	const [x, y] = [a as number | Limbs, b as number | Limbs];
	return lt(x, y) ? -1 : lt(y, x) ? 1 : 0;
}

/**
 * `Ord::cmp` of two strings, by code point as Rust's UTF-8 bytes sort. JS's `<` compares
 * UTF-16 units, which puts a character past U+FFFF, a surrogate pair, below U+E000.
 */
export function cmpStr(a: string, b: string): Ordering {
	if (a === b) return 0;
	const n = Math.min(a.length, b.length);
	for (let i = 0; i < n; i++) {
		let x = a.charCodeAt(i);
		let y = b.charCodeAt(i);
		if (x === y) continue;
		if (x >= 0xd800 && y >= 0xd800) {
			x = x >= 0xe000 ? x - 0x800 : x + 0x2000;
			y = y >= 0xe000 ? y - 0x800 : y + 0x2000;
		}
		return x < y ? -1 : 1;
	}
	return a.length < b.length ? -1 : 1;
}

/**
 * `Ord::cmp` of tuples, or `PartialOrd::partial_cmp`, from each field's: the first that
 * is not `Equal`, as `None` is not.
 */
export function cmpTuple<T extends readonly unknown[], O extends Ordering | undefined>(fields: readonly ((a: never, b: never) => O)[]): (a: T, b: T) => O {
	return (a, b) => {
		for (let i = 0; i < fields.length; i++) {
			const c = (fields[i] as (a: unknown, b: unknown) => O)(a[i], b[i]);
			if (c !== 0) return c;
		}
		return 0 as O;
	};
}

/**
 * `PartialOrd::lt` of tuples, or `le`, `gt`, or `ge`: `test` of their last fields, unless
 * `cmps`, the `partial_cmp`s of the others, find one not `Equal` first, which decides
 * whether it is `order`, as `None` is not.
 */
export function testTuple<T extends readonly unknown[]>(
	cmps: readonly ((a: never, b: never) => Ordering | undefined)[],
	test: (a: never, b: never) => boolean,
	order: Ordering,
): (a: T, b: T) => boolean {
	return (a, b) => {
		for (let i = 0; i < cmps.length; i++) {
			const c = (cmps[i] as (a: unknown, b: unknown) => Ordering | undefined)(a[i], b[i]);
			if (c !== 0) return c === order;
		}
		return (test as (a: unknown, b: unknown) => boolean)(a[cmps.length], b[cmps.length]);
	};
}

/** `PartialOrd::lt` by `cmp`, a `partial_cmp`, as the trait's default tests it: whether it is `Less`. */
export function cmpLt<T>(cmp: (a: T, b: T) => Ordering | undefined): (a: T, b: T) => boolean {
	return (a, b) => cmp(a, b) === -1;
}

/** `PartialOrd::le` by `cmp`, a `partial_cmp`: whether it is `Less` or `Equal`. */
export function cmpLe<T>(cmp: (a: T, b: T) => Ordering | undefined): (a: T, b: T) => boolean {
	return (a, b) => {
		const c = cmp(a, b);
		return c !== undefined && c <= 0;
	};
}

/** `PartialOrd::gt` by `cmp`, a `partial_cmp`: whether it is `Greater`. */
export function cmpGt<T>(cmp: (a: T, b: T) => Ordering | undefined): (a: T, b: T) => boolean {
	return (a, b) => cmp(a, b) === 1;
}

/** `PartialOrd::ge` by `cmp`, a `partial_cmp`: whether it is `Greater` or `Equal`. */
export function cmpGe<T>(cmp: (a: T, b: T) => Ordering | undefined): (a: T, b: T) => boolean {
	return (a, b) => {
		const c = cmp(a, b);
		return c !== undefined && c >= 0;
	};
}

/** `Ord::cmp` of `Option`s, or `PartialOrd::partial_cmp`, from their values': `None` first. */
export function cmpOption<T, O extends Ordering | undefined>(cmp: (a: T, b: T) => O): (a: T | undefined, b: T | undefined) => O;
export function cmpOption<T, O extends Ordering | undefined>(cmp: (a: T, b: T) => O, none: None): (a: T | None, b: T | None) => O;
export function cmpOption<T, O extends Ordering | undefined>(
	cmp: (a: T, b: T) => O,
	none?: None,
): (a: T | undefined | None, b: T | undefined | None) => O {
	return (a, b) => (a === none ? (b === none ? 0 : -1) : b === none ? 1 : cmp(a as T, b as T)) as O;
}

/**
 * `Ord::cmp` of `Reverse`s, or `PartialOrd::partial_cmp` or one of its tests: their
 * values', the other way.
 */
export function cmpReverse<T, O>(cmp: (a: T, b: T) => O): (a: T, b: T) => O {
	return (a, b) => cmp(b, a);
}

/**
 * `Ord::cmp` of slices, or `PartialOrd::partial_cmp`, by their elements' in order, then
 * by length.
 */
export function cmpSlice<T, O extends Ordering | undefined>(cmp: (a: T, b: T) => O): (a: ArrayLike<T>, b: ArrayLike<T>) => O {
	return (a, b) => {
		const n = Math.min(a.length, b.length);
		for (let i = 0; i < n; i++) {
			const c = cmp(a[i], b[i]);
			if (c !== 0) return c;
		}
		return (a.length < b.length ? -1 : a.length > b.length ? 1 : 0) as O;
	};
}

/** `Ord::cmp` of `Arc`s, or `PartialOrd::partial_cmp` or one of its tests: their values'. */
export function cmpArc<T, O>(cmp: (a: T, b: T) => O): (a: Arc<T>, b: Arc<T>) => O {
	return (a, b) => cmp(a.value, b.value);
}

/**
 * `Ord::clamp`, by `PartialOrd`'s `le`, `lt`, and `gt`: `min` if `a` is less, `max` if
 * greater, or else `a`, once `min <= max` is asserted. The operands it does not return
 * drop, the last first, as std's do.
 */
export function ordClamp<T>(
	a: T,
	min: T,
	max: T,
	le: (a: T, b: T) => boolean,
	lt: (a: T, b: T) => boolean,
	gt: (a: T, b: T) => boolean,
	drop?: (value: T) => void,
): T {
	if (!le(min, max)) panic("assertion failed: min <= max");
	const out = lt(a, min) ? 1 : gt(a, max) ? 2 : 0;
	if (drop !== undefined) dropAll([max, min, a].filter((_, i) => 2 - i !== out), drop);
	return out === 1 ? min : out === 2 ? max : a;
}

/** `Ord::max`, by `lt`: `a` if `b < a`, or else `b`, and the other drops. */
export function ordMax<T>(a: T, b: T, lt: (a: T, b: T) => boolean, drop?: (value: T) => void): T {
	if (lt(b, a)) {
		drop?.(b);
		return a;
	}
	drop?.(a);
	return b;
}

/** `Ord::min`, by `lt`: `b` if `b < a`, or else `a`, and the other drops. */
export function ordMin<T>(a: T, b: T, lt: (a: T, b: T) => boolean, drop?: (value: T) => void): T {
	if (lt(b, a)) {
		drop?.(a);
		return b;
	}
	drop?.(b);
	return a;
}

/** `Ordering::reverse`. */
export function orderingReverse(o: Ordering): Ordering {
	return o === 0 ? 0 : o === 1 ? -1 : 1;
}

/** `Ordering::then`, and with `lazy`, `then_with`, whose `other` is a function. */
export function orderingThen(o: Ordering, other: Ordering | (() => Ordering), lazy = false): Ordering {
	if (o !== 0) return o;
	return lazy ? (other as () => Ordering)() : (other as Ordering);
}

// Equality

/** `PartialEq::eq` of two values JS compares as Rust does: numbers, strings, booleans. */
export function same<T>(a: T, b: T): boolean {
	return a === b;
}

/** `PartialEq::eq` of tuples, from each field's. */
export function eqTuple<T extends readonly unknown[]>(fields: readonly ((a: never, b: never) => boolean)[]): (a: T, b: T) => boolean {
	return (a, b) => {
		for (let i = 0; i < fields.length; i++) if (!(fields[i] as (a: unknown, b: unknown) => boolean)(a[i], b[i])) return false;
		return true;
	};
}

/** `PartialEq::eq` of slices: their lengths', then their elements' in order. */
export function eqSlice<T>(eq: (a: T, b: T) => boolean): (a: ArrayLike<T>, b: ArrayLike<T>) => boolean {
	return (a, b) => {
		if (a.length !== b.length) return false;
		for (let i = 0; i < a.length; i++) if (!eq(a[i], b[i])) return false;
		return true;
	};
}

/** `PartialEq::eq` of `Option`s, from their values'. */
export function eqOption<T>(eq: (a: T, b: T) => boolean): (a: T | undefined, b: T | undefined) => boolean;
export function eqOption<T>(eq: (a: T, b: T) => boolean, none: None): (a: T | None, b: T | None) => boolean;
export function eqOption<T>(
	eq: (a: T, b: T) => boolean,
	none?: None,
): (a: T | undefined | None, b: T | undefined | None) => boolean {
	return (a, b) => (a === none || b === none ? a === b : eq(a as T, b as T));
}

/**
 * `PartialEq::eq` of `Arc`s: their values', after, with `ident`, whether they are one
 * `Arc`, which std checks first of a value that is `Eq`.
 */
export function eqArc<T>(eq: (a: T, b: T) => boolean, ident = false): (a: Arc<T>, b: Arc<T>) => boolean {
	return (a, b) => (ident && a === b) || eq(a.value, b.value);
}

// Hashing

/**
 * `DefaultHasher`: a running hash of the words `Hash` writes. Its values are its own,
 * as std leaves `DefaultHasher`'s unspecified; equal keys write equal words.
 */
export interface Hasher {
	h: number;
}

/** `DefaultHasher::new`. */
export function hasher(): Hasher {
	return { h: 0 };
}

function mix(s: Hasher, word: number): void {
	s.h = Math.imul(((s.h << 5) | (s.h >>> 27)) ^ word, 0x9e3779b1);
}

/** `Hash::hash` of an integer, or of a `bool`. */
export function hashInt<T extends number | boolean | Limbs>(value: T, s: Hasher): void {
	const n: number | boolean | Limbs = value;
	if (typeof n === "boolean") return mix(s, n ? 1 : 0);
	if (typeof n === "number") {
		if ((n | 0) === n) return mix(s, n);
		mix(s, n >>> 0);
		return mix(s, Math.floor(n / 4294967296) | 0);
	}
	mix(s, n.lo);
	if ("w1" in n) {
		mix(s, n.w1);
		mix(s, n.w2);
	}
	mix(s, n.hi);
}

/** `Hash::hash` of a string: its length, then its UTF-16 units. */
export function hashStr(str: string, s: Hasher): void {
	mix(s, str.length);
	for (let i = 0; i < str.length; i++) mix(s, str.charCodeAt(i));
}

/** `Hash::hash` of tuples, from each field's. */
export function hashTuple<T extends readonly unknown[]>(fields: readonly ((value: never, s: Hasher) => void)[]): (value: T, s: Hasher) => void {
	return (value, s) => {
		for (let i = 0; i < fields.length; i++) (fields[i] as (value: unknown, s: Hasher) => void)(value[i], s);
	};
}

/** `Hash::hash` of a slice: its length, then its elements' in order. */
export function hashSlice<T>(hash: (value: T, s: Hasher) => void): (value: ArrayLike<T>, s: Hasher) => void {
	return (value, s) => {
		mix(s, value.length);
		for (let i = 0; i < value.length; i++) hash(value[i], s);
	};
}

/** `Hash::hash` of an `Option`: whether it is `Some`, then its value's. */
export function hashOption<T>(hash: (value: T, s: Hasher) => void): (value: T | undefined, s: Hasher) => void;
export function hashOption<T>(hash: (value: T, s: Hasher) => void, none: None): (value: T | None, s: Hasher) => void;
export function hashOption<T>(
	hash: (value: T, s: Hasher) => void,
	none?: None,
): (value: T | undefined | None, s: Hasher) => void {
	return (value, s) => {
		mix(s, value === none ? 0 : 1);
		if (value !== none) hash(value as T, s);
	};
}

/** `Hash::hash` of an `Arc`: its value's. */
export function hashArc<T>(hash: (value: T, s: Hasher) => void): (value: Arc<T>, s: Hasher) => void {
	return (value, s) => hash(value.value, s);
}

/** `Hasher::finish`, as a JS map's key: a 30-bit integer, which V8 keeps unboxed. */
export function hashFinish(s: Hasher): number {
	return (s.h ^ (s.h >>> 15)) & 0x3fffffff;
}

/** How a hash collection finds a key that is a JS object: its `Hash` and `Eq`. */
export interface Hashing<K> {
	readonly hash: (key: K) => number;
	readonly eq: (a: K, b: K) => boolean;
	/** Whether a key that is a JS primitive goes by them too: an enum with its own `Eq`. */
	readonly all: boolean;
}

/** The `Hashing` of a key type, from its `Hash::hash` and `PartialEq::eq`. */
export function hashing<K>(hash: (key: K, s: Hasher) => void, eq: (a: K, b: K) => boolean, all = false): Hashing<K> {
	return {
		hash: (key) => {
			const s = hasher();
			hash(key, s);
			return hashFinish(s);
		},
		eq,
		all,
	};
}

/** A value as its own clone. */
function itself<T>(value: T): T {
	return value;
}

/** The place of index `i` of `a`. */
export class AtIndex<T> implements Place<T> {
	private readonly a: { [i: number]: T };
	private readonly i: number;

	constructor(a: { [i: number]: T }, i: number) {
		this.a = a;
		this.i = i;
	}

	get v(): T {
		return this.a[this.i];
	}

	set v(value: T) {
		this.a[this.i] = value;
	}
}

/** The place of `k`'s value in `m`, which holds it. */
class AtKey<K, V> implements Place<V> {
	private readonly m: Map<K, V>;
	private readonly k: K;

	constructor(m: Map<K, V>, k: K) {
		this.m = m;
		this.k = k;
	}

	get v(): V {
		return this.m.get(this.k) as V;
	}

	set v(value: V) {
		this.m.set(this.k, value);
	}
}

// HashMap and HashSet

/** An entry whose key is a JS object, in the chain of its hash. */
interface Slot<K, V> {
	readonly k: K;
	v: V;
	next: Slot<K, V> | undefined;
}

/**
 * `HashMap<K, V>`, and `HashSet<K>` as a map to `undefined`. A key that is a JS primitive,
 * a number, string, or boolean, is its own JS map key, as Rust compares it; a key that
 * is an object goes by its hash, to a chain of the keys `Eq` tells apart. Iteration
 * visits the first kind in insertion order and then the second by hash, an order Rust
 * leaves unspecified.
 */
export interface HashMap<K, V> {
	p: Map<K, V>;
	o: Map<number, Slot<K, V>> | undefined;
	/** How many keys `o` holds. */
	n: number;
	/** How to find a key that is an object, or `undefined` if no key is one. */
	readonly h: Hashing<K> | undefined;
}

/** `HashSet<K>`. */
export type HashSet<K> = HashMap<K, undefined>;

/** `HashMap::new` and `HashSet::new`, with how to find a key that is an object. */
export function hashNew<K, V>(h?: Hashing<K>): HashMap<K, V> {
	return { p: new Map(), o: undefined, n: 0, h };
}

/** Whether `key` goes by its hash rather than as its own JS map key. */
function hashed<K, V>(m: HashMap<K, V>, key: K): boolean {
	return m.h !== undefined && (m.h.all || typeof key === "object");
}

/** The slot holding a key equal to `key`, a hashed key whose hash is `hash` if given. */
function slot<K, V>(m: HashMap<K, V>, key: K, hash?: number): Slot<K, V> | undefined {
	const h = m.h as Hashing<K>;
	for (let s = m.o?.get(hash ?? h.hash(key)); s !== undefined; s = s.next) if (h.eq(s.k, key)) return s;
	return undefined;
}

export function hashLen<K, V>(m: HashMap<K, V>): number {
	return m.p.size + m.n;
}

/** `HashMap::get`: the value there, or else `none`, the `None` of an `Option` of a value that can be `undefined`. */
export function hashGet<K, V>(m: HashMap<K, V>, key: K): V | undefined;
export function hashGet<K, V>(m: HashMap<K, V>, key: K, none: None): V | None;
export function hashGet<K, V>(m: HashMap<K, V>, key: K, none?: None): V | None | undefined {
	if (hashed(m, key)) {
		const s = slot(m, key);
		return s === undefined ? none : s.v;
	}
	const v = m.p.get(key);
	return v !== undefined || none === undefined || m.p.has(key) ? v : none;
}

/** `HashMap::get_mut` of a value JS copies: its place, if the map holds `key`. */
export function hashGetMut<K, V>(m: HashMap<K, V>, key: K): Place<V> | undefined {
	if (hashed(m, key)) return slot(m, key);
	return m.p.has(key) ? new AtKey(m.p, key) : undefined;
}

export function hashContains<K, V>(m: HashMap<K, V>, key: K): boolean {
	return hashed(m, key) ? slot(m, key) !== undefined : m.p.has(key);
}

/** `HashMap::index`: `map[&key]`. */
export function hashIndex<K, V>(m: HashMap<K, V>, key: K): V {
	if (!hashContains(m, key)) panic("no entry found for key");
	return hashGet(m, key) as V;
}

/** Drop `key` by `k` and then `value` by `v`. */
function dropPair<K, V>(key: K, value: V, k?: (key: K) => void, v?: (value: V) => void): void {
	k?.(key);
	v?.(value);
}

/** Drop each key by `k` and its value by `v`, in order. */
function dropPairs<K, V>(keys: readonly K[], values: readonly V[], k?: (key: K) => void, v?: (value: V) => void): void {
	for (let i = 0; i < keys.length; i++) dropPair(keys[i], values[i], k, v);
}

/** `HashMap::insert`: the old value, or else `none`. An equal key already there stays, and `key` drops. */
export function hashInsert<K, V>(m: HashMap<K, V>, key: K, value: V, k?: (key: K) => void): V | undefined;
export function hashInsert<K, V>(m: HashMap<K, V>, key: K, value: V, k: ((key: K) => void) | undefined, none: None): V | None;
export function hashInsert<K, V>(m: HashMap<K, V>, key: K, value: V, k?: (key: K) => void, none?: None): V | None | undefined {
	if (!hashed(m, key)) {
		const had = m.p.has(key);
		const old = m.p.get(key);
		m.p.set(key, value);
		if (!had) return none;
		k?.(key);
		return old;
	}
	// Hashed once, as std's is, into an empty map too.
	const hash = (m.h as Hashing<K>).hash(key);
	const found = slot(m, key, hash);
	if (found !== undefined) {
		const old = found.v;
		found.v = value;
		k?.(key);
		return old;
	}
	add(m, key, value, hash);
	return none;
}

/** Add a hashed key that is not there yet, whose hash is `hash`: its new slot. */
function add<K, V>(m: HashMap<K, V>, key: K, value: V, hash = (m.h as Hashing<K>).hash(key)): Slot<K, V> {
	m.o ??= new Map();
	const s = { k: key, v: value, next: m.o.get(hash) };
	m.o.set(hash, s);
	m.n++;
	return s;
}

/** `HashSet::insert`: whether `key` was not there yet. An equal key already there stays, and `key` drops. */
export function hashAdd<K>(m: HashSet<K>, key: K, k?: (key: K) => void): boolean {
	if (!hashed(m, key)) {
		if (m.p.has(key)) {
			k?.(key);
			return false;
		}
		m.p.set(key, undefined);
		return true;
	}
	const hash = (m.h as Hashing<K>).hash(key);
	if (slot(m, key, hash) !== undefined) {
		k?.(key);
		return false;
	}
	add(m, key, undefined, hash);
	return true;
}

/** `HashMap::remove_entry`: the key and value there, if any. */
export function hashRemoveEntry<K, V>(m: HashMap<K, V>, key: K): [K, V] | undefined {
	if (!hashed(m, key)) {
		if (!m.p.has(key)) return undefined;
		const value = m.p.get(key) as V;
		m.p.delete(key);
		return [key, value];
	}
	const hash = (m.h as Hashing<K>).hash(key);
	const s = slot(m, key, hash);
	if (s === undefined) return undefined;
	unlink(m, hash, s);
	return [s.k, s.v];
}

/** Take `s`, a slot whose key's hash is `hash`, out of `m`. */
function unlink<K, V>(m: HashMap<K, V>, hash: number, s: Slot<K, V>): void {
	const o = m.o as Map<number, Slot<K, V>>;
	let prev = o.get(hash) as Slot<K, V>;
	if (prev === s) {
		if (s.next !== undefined) o.set(hash, s.next);
		else o.delete(hash);
	} else {
		while (prev.next !== s) prev = prev.next as Slot<K, V>;
		prev.next = s.next;
	}
	m.n--;
}

/** `HashMap::remove`: the value there, whose key drops, or else `none`. */
export function hashRemove<K, V>(m: HashMap<K, V>, key: K, k?: (key: K) => void): V | undefined;
export function hashRemove<K, V>(m: HashMap<K, V>, key: K, k: ((key: K) => void) | undefined, none: None): V | None;
export function hashRemove<K, V>(m: HashMap<K, V>, key: K, k?: (key: K) => void, none?: None): V | None | undefined {
	if (!hashed(m, key)) {
		if (!m.p.has(key)) return none;
		const old = m.p.get(key);
		m.p.delete(key);
		k?.(key);
		return old;
	}
	const entry = hashRemoveEntry(m, key);
	if (entry === undefined) return none;
	k?.(entry[0]);
	return entry[1];
}

/** `HashSet::remove`: whether `key` was there. The key it held drops. */
export function hashDelete<K>(m: HashSet<K>, key: K, k?: (key: K) => void): boolean {
	if (!hashed(m, key)) {
		if (!m.p.delete(key)) return false;
		k?.(key);
		return true;
	}
	const entry = hashRemoveEntry(m, key);
	if (entry !== undefined) k?.(entry[0]);
	return entry !== undefined;
}

/** `HashSet::get`, and `HashSet::take` if `take`: the equal key the set holds. */
export function hashKey<K>(m: HashSet<K>, key: K, take = false): K | undefined {
	if (hashed(m, key)) return take ? hashRemoveEntry(m, key)?.[0] : slot(m, key)?.k;
	if (!m.p.has(key)) return undefined;
	if (take) m.p.delete(key);
	return key;
}

/** Drop `m`: each key by `k` and each value by `v`, of those that drop, entry by entry. */
export function hashDrop<K, V>(m: HashMap<K, V>, k: ((key: K) => void) | undefined, v?: (value: V) => void): void {
	for (const [key, value] of m.p) dropPair(key, value, k, v);
	for (const first of m.o?.values() ?? []) {
		for (let s: Slot<K, V> | undefined = first; s !== undefined; s = s.next) dropPair(s.k, s.v, k, v);
	}
}

/** `clear`, which drops each entry and leaves the map empty. */
export function hashClear<K, V>(m: HashMap<K, V>, k?: (key: K) => void, v?: (value: V) => void): void {
	if (k !== undefined || v !== undefined) hashDrop(m, k, v);
	m.p.clear();
	m.o = undefined;
	m.n = 0;
}

/** `retain`, which keeps each entry `f` returns true for. Each entry goes, and drops, as `f` rejects it. */
export function hashRetain<K, V>(m: HashMap<K, V>, f: (key: K, value: V) => boolean, k?: (key: K) => void, v?: (value: V) => void): void {
	retainHash(m, f as (key: K, value: V | Place<V>) => boolean, false, k, v);
}

/** `retain` of values JS copies: `f` takes the place of each. */
export function hashRetainMut<K, V>(m: HashMap<K, V>, f: (key: K, value: Place<V>) => boolean, k?: (key: K) => void, v?: (value: V) => void): void {
	retainHash(m, f as (key: K, value: V | Place<V>) => boolean, true, k, v);
}

/** `retain`, handing `f` each value or, if `lend`, its place. */
function retainHash<K, V>(m: HashMap<K, V>, f: (key: K, value: V | Place<V>) => boolean, lend: boolean, k?: (key: K) => void, v?: (value: V) => void): void {
	for (const [key, value] of m.p) {
		if (f(key, lend ? new AtKey(m.p, key) : value)) continue;
		const now = lend ? (m.p.get(key) as V) : value;
		m.p.delete(key);
		dropPair(key, now, k, v);
	}
	if (m.o === undefined) return;
	for (const [hash, first] of m.o) {
		let prev: Slot<K, V> | undefined;
		for (let s: Slot<K, V> | undefined = first; s !== undefined; s = s.next) {
			if (f(s.k, lend ? s : s.v)) {
				prev = s;
				continue;
			}
			m.n--;
			if (prev !== undefined) prev.next = s.next;
			else if (s.next !== undefined) m.o.set(hash, s.next);
			else m.o.delete(hash);
			dropPair(s.k, s.v, k, v);
		}
	}
}

/**
 * What a collection's iterator yields: its entries as `[key, value]`, keys, values, or
 * `[key, place]`, the place of a value JS copies.
 */
type Pick = 0 | 1 | 2 | 3;
const ENTRIES = 0;
const KEYS = 1;
const VALUES = 2;
const PLACES = 3;

/**
 * An iterator of a hash collection: `iter`, `keys`, `values`, `drain`, and the rest.
 * One that owns its entries is given what drops each key and value of those that drop,
 * and drops each entry it still holds as it drops.
 */
export class HashIter<K, V, T> extends Iter<T> implements Disposable {
	private readonly m: Map<K, V>;
	private readonly p: Iterator<[K, V]>;
	private readonly o: Iterator<Slot<K, V>> | undefined;
	private at: Slot<K, V> | undefined;
	private readonly pick: Pick;
	left: number;
	private readonly k: ((key: K) => void) | undefined;
	private readonly v: ((value: V) => void) | undefined;
	/** Whether it is a `drain`, whose `count` reads each entry, as an `into_iter`'s does not. */
	readonly drain: boolean;

	constructor(m: HashMap<K, V>, pick: Pick, k?: (key: K) => void, v?: (value: V) => void, drain = false) {
		super();
		this.m = m.p;
		this.p = m.p.entries();
		this.o = m.o?.values();
		this.pick = pick;
		this.left = hashLen(m);
		this.k = k;
		this.v = v;
		this.drain = drain;
	}

	/** The next entry, of the `left` it has. */
	private entry(): [K, V] {
		const entry = this.p.next();
		if (!entry.done) return entry.value;
		const s = this.slot();
		return [s.k, s.v];
	}

	/** The next entry of the `left` it has, as its key and the place of its value. */
	private place(): [K, Place<V>] {
		const entry = this.p.next();
		if (!entry.done) return [entry.value[0], new AtKey(this.m, entry.value[0])];
		const s = this.slot();
		return [s.k, s];
	}

	/** The next chained entry, once the primitive keys are done. */
	private slot(): Slot<K, V> {
		if (this.at === undefined) this.at = (this.o as Iterator<Slot<K, V>>).next().value;
		const s = this.at as Slot<K, V>;
		this.at = s.next;
		return s;
	}

	next(): T | Done {
		if (this.left === 0) return DONE;
		this.left--;
		if (this.pick === PLACES) return this.place() as T;
		const entry = this.entry();
		if (this.pick === ENTRIES) return entry as T;
		return (this.pick === KEYS ? entry[0] : entry[1]) as unknown as T;
	}

	override len(): number {
		return this.left;
	}

	[Symbol.dispose](): void {
		if (this.k === undefined && this.v === undefined) return;
		const n = this.left;
		this.left = 0;
		for (let i = 0; i < n; i++) {
			const [key, value] = this.entry();
			dropPair(key, value, this.k, this.v);
		}
	}
}

/**
 * `iter` and `iter_mut` of a map, and with what drops each key and value of those that
 * drop, `into_iter`: its entries, as `[key, value]`.
 */
export function hashIter<K, V>(m: HashMap<K, V>, k?: (key: K) => void, v?: (value: V) => void): Iter<[K, V]> {
	return new HashIter<K, V, [K, V]>(m, ENTRIES, k, v);
}

/** `keys` of a map and `iter` of a set, and with what drops each key, `into_keys` of values that don't drop and `into_iter`. */
export function hashKeys<K, V>(m: HashMap<K, V>, k?: (key: K) => void): Iter<K> {
	return new HashIter<K, V, K>(m, KEYS, k);
}

/** `values` and `values_mut`, and with what drops each value, `into_values` of keys that don't drop. */
export function hashValues<K, V>(m: HashMap<K, V>, v?: (value: V) => void): Iter<V> {
	return new HashIter<K, V, V>(m, VALUES, undefined, v);
}

/** `iter_mut` of values JS copies: each key and the place of its value, in `hashIter`'s order. */
export function hashIterMut<K, V>(m: HashMap<K, V>): Iter<[K, Place<V>]> {
	return new HashIter<K, V, [K, Place<V>]>(m, PLACES);
}

/** `values_mut` of values JS copies: the place of each. */
export function hashValuesMut<K, V>(m: HashMap<K, V>): Iter<Place<V>> {
	return iterMap(hashIterMut(m), (entry) => entry[1]);
}

/** `drain` of a map: every entry, taken out at once, which drops as `hashIter`'s. */
export function hashDrain<K, V>(m: HashMap<K, V>, k?: (key: K) => void, v?: (value: V) => void): Iter<[K, V]> {
	return new HashIter<K, V, [K, V]>(taken(m), ENTRIES, k, v, true);
}

/** `drain` of a set: every key, taken out at once. */
export function hashDrainKeys<K, V>(m: HashMap<K, V>, k?: (key: K) => void): Iter<K> {
	return new HashIter<K, V, K>(taken(m), KEYS, k, undefined, true);
}

/** What `m` held, which it no longer does. */
function taken<K, V>(m: HashMap<K, V>): HashMap<K, V> {
	const out: HashMap<K, V> = { p: m.p, o: m.o, n: m.n, h: m.h };
	m.p = new Map();
	hashClear(m);
	return out;
}

/** Extend `c` by `add` of each of `items`, taken as `take` says, as std's `extend` takes the iterator. */
function extending<C, T>(c: C, items: Iter<T>, add: (item: T) => void, take?: Take): C {
	if (take === "fold") items.fold<void>(undefined, (_, item) => add(item));
	else {
		// By `next` where std's reads by it, and else as its `try_fold` steps.
		const read = take === "next" ? items : items.folded();
		for (let item = read.next(); item !== DONE; item = read.next()) add(item);
	}
	if (take === "next") dispose(items);
	return c;
}

/**
 * `extend` of a map with `[key, value]`s, and `collect` into one through `hashNew`. Of
 * equal keys, the first stays and each later one drops, then the value it replaced.
 */
export function hashExtend<K, V>(m: HashMap<K, V>, items: Iter<[K, V]>, k?: (key: K) => void, v?: (value: V) => void, take?: Take): HashMap<K, V> {
	const add = (item: [K, V]) => {
		const old = hashInsert(m, item[0], item[1], k);
		if (old !== undefined) v?.(old);
	};
	return extending(m, items, add, take);
}

/** `extend` of a set with keys, and `collect` into one, as `hashExtend`: a key already there drops. */
export function hashExtendKeys<K>(m: HashSet<K>, keys: Iter<K>, k?: (key: K) => void, take?: Take): HashSet<K> {
	return extending(m, keys, (key) => hashAdd(m, key, k), take);
}

/** `clone`, with each key and value through its `Clone`, or as itself without one. */
export function hashClone<K, V>(m: HashMap<K, V>, key: (k: K) => K = itself, value: (v: V) => V = itself): HashMap<K, V> {
	const out = hashNew<K, V>(m.h);
	for (const [k, v] of m.p) out.p.set(key(k), value(v));
	for (const first of m.o?.values() ?? []) {
		for (let s: Slot<K, V> | undefined = first; s !== undefined; s = s.next) add(out, key(s.k), value(s.v));
	}
	return out;
}

// BTreeMap and BTreeSet

/**
 * The most keys a node holds. A B+ tree's leaves hold its keys and values in order, and
 * its branches each child's lower bound, so every operation but a split or merge shifts
 * at most this many array slots after `log_{NODE/2} n` searches of one node each.
 */
const NODE = 64;
/** The fewest keys a node other than the root holds, half of `NODE`, as a literal a bundler can drop. */
const MIN = 32;

/** A leaf: its keys in order, each key's value at the same index, and the leaves beside it. */
interface Leaf<K, V> {
	k: K[];
	v: V[];
	prev: Leaf<K, V> | undefined;
	next: Leaf<K, V> | undefined;
}

/** A branch: its children, and each one's lower bound, which the first child's is not. */
interface Branch<K, V> {
	k: K[];
	c: Node<K, V>[];
}

type Node<K, V> = Leaf<K, V> | Branch<K, V>;

/** `BTreeMap<K, V>`, and `BTreeSet<K>` as a map to `undefined`: a B+ tree ordered by `Ord`. */
export interface BTreeMap<K, V> {
	/** The root, which is a leaf where `h` is 0. */
	t: Node<K, V>;
	/** The height: how many branches are above each leaf. */
	h: number;
	/** How many keys it holds. */
	n: number;
	/** `Ord::cmp` of the keys. */
	readonly c: (a: K, b: K) => Ordering;
	/**
	 * Whether std's map would have its root node: from its first insert until `clear`.
	 * Only a map with one checks a range's bounds.
	 */
	r: boolean;
}

/** `BTreeSet<K>`. */
export type BTreeSet<K> = BTreeMap<K, undefined>;

function leaf<K, V>(k: K[] = [], v: V[] = []): Leaf<K, V> {
	return { k, v, prev: undefined, next: undefined };
}

/** `BTreeMap::new` and `BTreeSet::new`, ordered by `cmp`. */
export function btreeNew<K, V>(cmp: (a: K, b: K) => Ordering): BTreeMap<K, V> {
	return { t: leaf(), h: 0, n: 0, c: cmp, r: false };
}

/** The index of `key` in `keys` if there, or else the bitwise not of where it would go. */
function search<K>(keys: K[], key: K, cmp: (a: K, b: K) => Ordering): number {
	let lo = 0;
	let hi = keys.length;
	while (lo < hi) {
		const mid = (lo + hi) >>> 1;
		const c = cmp(keys[mid], key);
		if (c === 0) return mid;
		if (c < 0) lo = mid + 1;
		else hi = mid;
	}
	return ~lo;
}

/** The index of the child of `b` whose keys `key` would be among. */
function child<K, V>(b: Branch<K, V>, key: K, cmp: (a: K, b: K) => Ordering): number {
	let lo = 1;
	let hi = b.k.length;
	while (lo < hi) {
		const mid = (lo + hi) >>> 1;
		if (cmp(b.k[mid], key) <= 0) lo = mid + 1;
		else hi = mid;
	}
	return lo - 1;
}

/** The leaf `key` would be in. */
function leafOf<K, V>(m: BTreeMap<K, V>, key: K): Leaf<K, V> {
	let node = m.t;
	for (let h = m.h; h > 0; h--) {
		const b = node as Branch<K, V>;
		node = b.c[child(b, key, m.c)];
	}
	return node as Leaf<K, V>;
}

/** The first leaf, or with `last`, the last. */
function end<K, V>(m: BTreeMap<K, V>, last: boolean): Leaf<K, V> {
	let node = m.t;
	for (let h = m.h; h > 0; h--) {
		const c = (node as Branch<K, V>).c;
		node = c[last ? c.length - 1 : 0];
	}
	return node as Leaf<K, V>;
}

export function btreeLen<K, V>(m: BTreeMap<K, V>): number {
	return m.n;
}

/** `BTreeMap::get`: the value there, or else `none`, the `None` of an `Option` of a value that can be `undefined`. */
export function btreeGet<K, V>(m: BTreeMap<K, V>, key: K): V | undefined;
export function btreeGet<K, V>(m: BTreeMap<K, V>, key: K, none: None): V | None;
export function btreeGet<K, V>(m: BTreeMap<K, V>, key: K, none?: None): V | None | undefined {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	return i < 0 ? none : l.v[i];
}

/** `BTreeMap::get_mut` of a value JS copies: its place, if the map holds `key`. */
export function btreeGetMut<K, V>(m: BTreeMap<K, V>, key: K): Place<V> | undefined {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	return i < 0 ? undefined : new AtIndex(l.v, i);
}

/** `BTreeMap::get_key_value`, and `BTreeSet::get`, which return the equal key the map holds. */
export function btreeGetKeyValue<K, V>(m: BTreeMap<K, V>, key: K): [K, V] | undefined {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	return i < 0 ? undefined : [l.k[i], l.v[i]];
}

export function btreeContains<K, V>(m: BTreeMap<K, V>, key: K): boolean {
	return search(leafOf(m, key).k, key, m.c) >= 0;
}

/** `BTreeMap::index`: `map[&key]`. */
export function btreeIndex<K, V>(m: BTreeMap<K, V>, key: K): V {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	if (i < 0) panic("no entry found for key");
	return l.v[i];
}

/** `BTreeMap::insert`: the old value, or else `none`. An equal key already there stays, and `key` drops. */
export function btreeInsert<K, V>(m: BTreeMap<K, V>, key: K, value: V, k?: (key: K) => void): V | undefined;
export function btreeInsert<K, V>(m: BTreeMap<K, V>, key: K, value: V, k: ((key: K) => void) | undefined, none: None): V | None;
export function btreeInsert<K, V>(m: BTreeMap<K, V>, key: K, value: V, k?: (key: K) => void, none?: None): V | None | undefined {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	if (i < 0) {
		insertAt(m, l, ~i, key, value);
		return none;
	}
	k?.(key);
	const old = l.v[i];
	l.v[i] = value;
	return old;
}

/** Puts `key` and `value` at index `i` of `l`, the leaf it goes in, and splits what overflows. */
function insertAt<K, V>(m: BTreeMap<K, V>, l: Leaf<K, V>, i: number, key: K, value: V): void {
	m.r = true;
	m.n++;
	put(l.k, i, key);
	put(l.v, i, value);
	if (l.k.length > NODE) settle(m, key);
}

/** Puts `item` at index `i` of `a`, after moving the rest up: in a node, faster than `splice`. */
function put<T>(a: T[], i: number, item: T): void {
	let j = a.length;
	a.push(item);
	for (; j > i; j--) a[j] = a[j - 1];
	a[i] = item;
}

/** Takes index `i` out of `a`, moving the rest down. */
function take<T>(a: T[], i: number): void {
	const n = a.length - 1;
	for (let j = i; j < n; j++) a[j] = a[j + 1];
	a.pop();
}

/** `BTreeSet::insert`: whether `key` was not there yet. An equal key already there stays, and `key` drops. */
export function btreeAdd<K>(m: BTreeSet<K>, key: K, k?: (key: K) => void): boolean {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	if (i >= 0) {
		k?.(key);
		return false;
	}
	insertAt(m, l, ~i, key, undefined);
	return true;
}

/** Takes the key and value at index `i` of `l` out, and merges the leaf if too small. */
function removeAt<K, V>(m: BTreeMap<K, V>, l: Leaf<K, V>, i: number): void {
	const key = l.k[i];
	m.n--;
	take(l.k, i);
	take(l.v, i);
	if (l.k.length < MIN && m.h > 0) settle(m, key);
}

/** `BTreeMap::remove_entry`, and `BTreeSet::take`. */
export function btreeRemoveEntry<K, V>(m: BTreeMap<K, V>, key: K): [K, V] | undefined {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	if (i < 0) return undefined;
	const entry: [K, V] = [l.k[i], l.v[i]];
	removeAt(m, l, i);
	return entry;
}

/** `BTreeMap::remove`: the value there, whose key drops, or else `none`. */
export function btreeRemove<K, V>(m: BTreeMap<K, V>, key: K, k?: (key: K) => void): V | undefined;
export function btreeRemove<K, V>(m: BTreeMap<K, V>, key: K, k: ((key: K) => void) | undefined, none: None): V | None;
export function btreeRemove<K, V>(m: BTreeMap<K, V>, key: K, k?: (key: K) => void, none?: None): V | None | undefined {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	if (i < 0) return none;
	const held = l.k[i];
	const value = l.v[i];
	removeAt(m, l, i);
	k?.(held);
	return value;
}

/** `BTreeSet::remove`: whether `key` was there. The key it held drops. */
export function btreeDelete<K>(m: BTreeSet<K>, key: K, k?: (key: K) => void): boolean {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	if (i < 0) return false;
	const held = l.k[i];
	removeAt(m, l, i);
	k?.(held);
	return true;
}

/**
 * Splits or merges the node on `key`'s path that a change left too large or too small,
 * and each above it that this leaves so in turn, finding them again from the root.
 */
function settle<K, V>(m: BTreeMap<K, V>, key: K): void {
	if (m.h > 0) settleBelow(m, m.t as Branch<K, V>, m.h, key);
	const t = m.t;
	if (t.k.length > NODE) {
		const right = split(t, m.h === 0);
		m.t = { k: [t.k[0], right.k[0]], c: [t, right] };
		m.h++;
	} else if (m.h > 0 && (t as Branch<K, V>).c.length === 1) {
		m.t = (t as Branch<K, V>).c[0];
		m.h--;
	}
}

/** `settle` below `b`, a branch at height `h`. */
function settleBelow<K, V>(m: BTreeMap<K, V>, b: Branch<K, V>, h: number, key: K): void {
	const i = child(b, key, m.c);
	const c = b.c[i];
	if (h > 1) settleBelow(m, c as Branch<K, V>, h - 1, key);
	if (c.k.length > NODE) {
		const right = split(c, h === 1);
		b.k.splice(i + 1, 0, right.k[0]);
		b.c.splice(i + 1, 0, right);
	} else if (c.k.length < MIN) {
		const j = i > 0 ? i - 1 : i;
		balance(b, j, h === 1);
	}
}

/** Takes the second half of `node` into a new node beside it, and returns that. */
function split<K, V>(node: Node<K, V>, isLeaf: boolean): Node<K, V> {
	const mid = node.k.length >> 1;
	if (!isLeaf) {
		const b = node as Branch<K, V>;
		return { k: b.k.splice(mid), c: b.c.splice(mid) };
	}
	const l = node as Leaf<K, V>;
	const right = leaf(l.k.splice(mid), l.v.splice(mid));
	right.prev = l;
	right.next = l.next;
	if (l.next !== undefined) l.next.prev = right;
	l.next = right;
	return right;
}

/** Evens out children `j` and `j + 1` of `b`, one of which is too small, or merges them. */
function balance<K, V>(b: Branch<K, V>, j: number, isLeaf: boolean): void {
	const left = b.c[j];
	const right = b.c[j + 1];
	const total = left.k.length + right.k.length;
	if (total <= NODE) {
		left.k.push(...right.k);
		if (isLeaf) {
			const l = left as Leaf<K, V>;
			const r = right as Leaf<K, V>;
			l.v.push(...r.v);
			l.next = r.next;
			if (r.next !== undefined) r.next.prev = l;
		} else {
			(left as Branch<K, V>).c.push(...(right as Branch<K, V>).c);
		}
		b.k.splice(j + 1, 1);
		b.c.splice(j + 1, 1);
		return;
	}
	const half = total >> 1;
	if (left.k.length < half) {
		const n = half - left.k.length;
		left.k.push(...right.k.splice(0, n));
		if (isLeaf) (left as Leaf<K, V>).v.push(...(right as Leaf<K, V>).v.splice(0, n));
		else (left as Branch<K, V>).c.push(...(right as Branch<K, V>).c.splice(0, n));
	} else {
		const n = left.k.length - half;
		right.k.unshift(...left.k.splice(half, n));
		if (isLeaf) (right as Leaf<K, V>).v.unshift(...(left as Leaf<K, V>).v.splice(half, n));
		else (right as Branch<K, V>).c.unshift(...(left as Branch<K, V>).c.splice(half, n));
	}
	b.k[j + 1] = right.k[0];
}

/** The tree of `keys` and their `values`, in order and without equal keys, built whole. */
function build<K, V>(m: BTreeMap<K, V>, keys: K[], values: V[]): void {
	m.n = keys.length;
	// As few nodes per level as hold them, evened out, so each holds at least half.
	const chunks = (n: number) => {
		const count = Math.max(1, Math.ceil(n / NODE));
		return Array.from({ length: count }, (_, i) => [Math.floor((i * n) / count), Math.floor(((i + 1) * n) / count)]);
	};
	let level: Node<K, V>[] = [];
	let prev: Leaf<K, V> | undefined;
	for (const [from, to] of chunks(keys.length)) {
		const l = leaf(keys.slice(from, to), values.slice(from, to));
		l.prev = prev;
		if (prev !== undefined) prev.next = l;
		prev = l;
		level.push(l);
	}
	let h = 0;
	while (level.length > 1) {
		level = chunks(level.length).map(([from, to]) => {
			const c = level.slice(from, to);
			return { k: c.map((node) => node.k[0]), c };
		});
		h++;
	}
	m.t = level[0];
	m.h = h;
}

/** Each key in order, and each one's value. */
function flatten<K, V>(m: BTreeMap<K, V>): [K[], V[]] {
	const keys: K[] = [];
	const values: V[] = [];
	for (let l: Leaf<K, V> | undefined = end(m, false); l !== undefined; l = l.next) {
		keys.push(...l.k);
		values.push(...l.v);
	}
	return [keys, values];
}

/** `first_key_value`, and with `last`, `last_key_value`. */
export function btreeFirst<K, V>(m: BTreeMap<K, V>, last = false): [K, V] | undefined {
	if (m.n === 0) return undefined;
	const l = end(m, last);
	const i = last ? l.k.length - 1 : 0;
	return [l.k[i], l.v[i]];
}

/** `BTreeSet::first`, and with `last`, `BTreeSet::last`. */
export function btreeFirstKey<K, V>(m: BTreeMap<K, V>, last = false): K | undefined {
	if (m.n === 0) return undefined;
	const l = end(m, last);
	return l.k[last ? l.k.length - 1 : 0];
}

/** `pop_first`, and with `last`, `pop_last`. */
export function btreePop<K, V>(m: BTreeMap<K, V>, last = false): [K, V] | undefined {
	if (m.n === 0) return undefined;
	const l = end(m, last);
	const i = last ? l.k.length - 1 : 0;
	const entry: [K, V] = [l.k[i], l.v[i]];
	removeAt(m, l, i);
	return entry;
}

/** `BTreeSet::pop_first`, and with `last`, `pop_last`. */
export function btreePopKey<K, V>(m: BTreeMap<K, V>, last = false): K | undefined {
	return btreePop(m, last)?.[0];
}

/** Drop `m`: each key by `k` and each value by `v`, of those that drop, in key order. */
export function btreeDrop<K, V>(m: BTreeMap<K, V>, k: ((key: K) => void) | undefined, v?: (value: V) => void): void {
	const [keys, values] = flatten(m);
	dropPairs(keys, values, k, v);
}

/** `clear`, which leaves the map empty and then drops each entry in key order. */
export function btreeClear<K, V>(m: BTreeMap<K, V>, k?: (key: K) => void, v?: (value: V) => void): void {
	if (k !== undefined || v !== undefined) {
		const old: BTreeMap<K, V> = { t: m.t, h: m.h, n: m.n, c: m.c, r: m.r };
		btreeClear(m);
		btreeDrop(old, k, v);
		return;
	}
	m.t = leaf();
	m.h = 0;
	m.n = 0;
	m.r = false;
}

/**
 * `split_off` of a map or set: the entries from `key` on, moved in order into a tree of
 * their own. Like std's, the new tree has a root unless `m` was empty.
 */
export function btreeSplitOff<K, V>(m: BTreeMap<K, V>, key: K): BTreeMap<K, V> {
	const out = btreeNew<K, V>(m.c);
	if (m.n === 0) return out;
	const [keys, values] = flatten(m);
	const i = search(keys, key, m.c);
	const at = i < 0 ? ~i : i;
	build(out, keys.slice(at), values.slice(at));
	build(m, keys.slice(0, at), values.slice(0, at));
	out.r = true;
	return out;
}

/**
 * `retain`, which visits the keys in order and keeps each entry `f` returns true for,
 * dropping each other one as `f` rejects it.
 */
export function btreeRetain<K, V>(m: BTreeMap<K, V>, f: (key: K, value: V) => boolean, k?: (key: K) => void, v?: (value: V) => void): void {
	retainBTree(m, f as (key: K, value: V | Place<V>) => boolean, false, k, v);
}

/** `retain` of values JS copies: `f` takes the place of each. */
export function btreeRetainMut<K, V>(m: BTreeMap<K, V>, f: (key: K, value: Place<V>) => boolean, k?: (key: K) => void, v?: (value: V) => void): void {
	retainBTree(m, f as (key: K, value: V | Place<V>) => boolean, true, k, v);
}

/** `retain`, handing `f` each value or, if `lend`, its place, which the map is built from again. */
function retainBTree<K, V>(m: BTreeMap<K, V>, f: (key: K, value: V | Place<V>) => boolean, lend: boolean, k?: (key: K) => void, v?: (value: V) => void): void {
	const [keys, values] = flatten(m);
	let kept = 0;
	for (let i = 0; i < keys.length; i++) {
		const key = keys[i];
		const keep = f(key, lend ? new AtIndex(values, i) : values[i]);
		const value = values[i];
		if (!keep) {
			dropPair(key, value, k, v);
			continue;
		}
		keys[kept] = key;
		values[kept] = value;
		kept++;
	}
	if (kept < keys.length || lend) {
		keys.length = kept;
		values.length = kept;
		build(m, keys, values);
	}
}

/**
 * An iterator of a B-tree's entries, keys, or values, from index `fi` of leaf `fl` up to
 * before index `bi` of leaf `bl`. The front moves to the next leaf as it passes a leaf's
 * last key, and the back to the previous leaf as it passes a leaf's first key. One that
 * owns its entries drops as `HashIter` does.
 */
export class BTreeIter<K, V, T> extends Iter<T> implements Disposable {
	fl: Leaf<K, V>;
	fi: number;
	bl: Leaf<K, V>;
	bi: number;
	/** How many items are left, where it is known: an iterator of the whole map. */
	private n: number;
	private readonly pick: Pick;
	private readonly k: ((key: K) => void) | undefined;
	private readonly v: ((value: V) => void) | undefined;
	/** Whether its `last` reads from the back, as a map's `into_keys`' and `into_values`' do. */
	readonly back: boolean;

	constructor(pick: Pick, fl: Leaf<K, V>, fi: number, bl: Leaf<K, V>, bi: number, n = -1, k?: (key: K) => void, v?: (value: V) => void, back = false) {
		super();
		this.pick = pick;
		if (fi === fl.k.length && fl.next !== undefined) {
			fl = fl.next;
			fi = 0;
		}
		if (bi === 0 && bl.prev !== undefined) {
			bl = bl.prev;
			bi = bl.k.length;
		}
		this.fl = fl;
		this.fi = fi;
		this.bl = bl;
		this.bi = bi;
		this.n = n;
		this.k = k;
		this.v = v;
		this.back = back;
	}

	next(): T | Done {
		if (done(this)) return DONE;
		const l = this.fl;
		const i = this.fi++;
		if (this.fi === l.k.length && l.next !== undefined) {
			this.fl = l.next;
			this.fi = 0;
		}
		this.n--;
		return this.picked(l, i);
	}

	override nextBack(): T | Done {
		if (done(this)) return DONE;
		const l = this.bl;
		const i = --this.bi;
		if (this.bi === 0 && l.prev !== undefined) {
			this.bl = l.prev;
			this.bi = this.bl.k.length;
		}
		this.n--;
		return this.picked(l, i);
	}

	/**
	 * What it yields of index `i` of leaf `l`: of `into_keys` or `into_values`, the other
	 * half drops first.
	 */
	private picked(l: Leaf<K, V>, i: number): T {
		if (this.pick === ENTRIES) return [l.k[i], l.v[i]] as T;
		if (this.pick === PLACES) return [l.k[i], new AtIndex(l.v, i)] as T;
		if (this.pick === KEYS) {
			this.v?.(l.v[i]);
			return l.k[i] as unknown as T;
		}
		this.k?.(l.k[i]);
		return l.v[i] as unknown as T;
	}

	override len(): number {
		return this.n;
	}

	[Symbol.dispose](): void {
		if (this.k === undefined && this.v === undefined) return;
		const keys: K[] = [];
		const values: V[] = [];
		while (!done(this)) {
			const l = this.fl;
			keys.push(l.k[this.fi]);
			values.push(l.v[this.fi]);
			this.fi++;
			if (this.fi === l.k.length && l.next !== undefined) {
				this.fl = l.next;
				this.fi = 0;
			}
		}
		this.n = 0;
		dropPairs(keys, values, this.k, this.v);
	}
}

/** Whether `it`'s front has met its back. */
function done<K, V, T>(it: BTreeIter<K, V, T>): boolean {
	return it.fl === it.bl ? it.fi >= it.bi : it.fi === 0 && it.bl.next === it.fl && it.bi === it.bl.k.length;
}

function whole<K, V, T>(m: BTreeMap<K, V>, pick: Pick, k?: (key: K) => void, v?: (value: V) => void, back = false): Iter<T> {
	const last = end(m, true);
	return new BTreeIter<K, V, T>(pick, end(m, false), 0, last, last.k.length, m.n, k, v, back);
}

/**
 * `iter` and `iter_mut` of a map, and with what drops each key and value of those that
 * drop, `into_iter`: its entries in key order.
 */
export function btreeIter<K, V>(m: BTreeMap<K, V>, k?: (key: K) => void, v?: (value: V) => void): Iter<[K, V]> {
	return whole(m, ENTRIES, k, v);
}

/**
 * `keys` of a map and `iter` of a set, and with droppers as `btreeIter`'s, `into_iter` of
 * a set, or with `back`, `into_keys`.
 */
export function btreeKeys<K, V>(m: BTreeMap<K, V>, k?: (key: K) => void, v?: (value: V) => void, back = false): Iter<K> {
	return whole(m, KEYS, k, v, back);
}

/** `iter_mut` of values JS copies: each key and the place of its value, in key order. */
export function btreeIterMut<K, V>(m: BTreeMap<K, V>): Iter<[K, Place<V>]> {
	return whole(m, PLACES);
}

/** `values_mut` of values JS copies: the place of each. */
export function btreeValuesMut<K, V>(m: BTreeMap<K, V>): Iter<Place<V>> {
	return iterMap(btreeIterMut(m), (entry) => entry[1]);
}

/** `values` and `values_mut`, and with droppers as `btreeIter`'s, `into_values`. */
export function btreeValues<K, V>(m: BTreeMap<K, V>, k?: (key: K) => void, v?: (value: V) => void): Iter<V> {
	return whole(m, VALUES, k, v, k !== undefined || v !== undefined);
}

/**
 * A range of keys or indices, as its value is: `a..b`, `a..=b`, `a..` as `{ start }`, `..b`
 * as `{ end }`, or `..` as `undefined`.
 */
export type Bounds<K> =
	| { readonly start: K; readonly end: K }
	| RangeInclusive<K>
	| { readonly start: K }
	| { readonly end: K; readonly inclusive?: true }
	| undefined;

/** `RangeBounds`: the start, the end, and whether the end is in the range, as `..=end`'s is. */
function bounds<K>(r: Bounds<K>): [{ key: K } | undefined, { key: K } | undefined, boolean] {
	if (r === undefined) return [undefined, undefined, false];
	// Exhausted by wrapping, an inclusive range excludes its start, as its `end_bound` says.
	// A field test, not `instanceof`, so a bundle without `a..=b` leaves the class out.
	if ("exhausted" in r) return [{ key: r.lo }, { key: r.exhausted ? r.lo : r.hi }, !r.exhausted];
	return ["start" in r ? { key: r.start } : undefined, "end" in r ? { key: r.end } : undefined, "inclusive" in r];
}

/** `BTreeMap::range` and `range_mut`: the entries within `r`. */
export function btreeRange<K, V>(m: BTreeMap<K, V>, r: Bounds<K>): Iter<[K, V]> {
	return range(m, r, ENTRIES, "BTreeMap");
}

/** `range_mut` of values JS copies: each key within `r` and the place of its value. */
export function btreeRangeMut<K, V>(m: BTreeMap<K, V>, r: Bounds<K>): Iter<[K, Place<V>]> {
	return range(m, r, PLACES, "BTreeMap");
}

/** `BTreeSet::range`: the keys within `r`. */
export function btreeRangeKeys<K, V>(m: BTreeMap<K, V>, r: Bounds<K>): Iter<K> {
	return range(m, r, KEYS, "BTreeSet");
}

/** The items of `m` within `r`, checked as std checks it. */
function range<K, V, T>(m: BTreeMap<K, V>, r: Bounds<K>, pick: Pick, name: string): Iter<T> {
	if (!m.r) return new BTreeIter<K, V, T>(pick, m.t as Leaf<K, V>, 0, m.t as Leaf<K, V>, 0);
	const [start, stop, inclusive] = bounds(r);
	if (start !== undefined && stop !== undefined && m.c(start.key, stop.key) > 0) {
		panic(`range start is greater than range end in ${name}`);
	}
	const [fl, fi] = start === undefined ? [end(m, false), 0] : edge(m, start.key, false);
	const last = end(m, true);
	const [bl, bi] = stop === undefined ? [last, last.k.length] : edge(m, stop.key, inclusive);
	return new BTreeIter<K, V, T>(pick, fl, fi, bl, bi);
}

/** Where the first key past `key` is if `past`, or else the first key not below it. */
function edge<K, V>(m: BTreeMap<K, V>, key: K, past: boolean): [Leaf<K, V>, number] {
	const l = leafOf(m, key);
	const i = search(l.k, key, m.c);
	return [l, i < 0 ? ~i : past ? i + 1 : i];
}

/**
 * `extend` of a map with `[key, value]`s, which keeps an equal key already there and drops
 * the one given, then the value it replaced, and takes `items` as `take` says. With
 * `last`, `collect` and `from` of entries that drop nothing into an empty one, which keep
 * the last of equal keys, as `btreeCollect` does.
 */
export function btreeExtend<K, V>(
	m: BTreeMap<K, V>,
	items: Iter<[K, V]>,
	last = false,
	k?: (key: K) => void,
	v?: (value: V) => void,
	take?: Take,
): BTreeMap<K, V> {
	const add = (item: [K, V]) => {
		if (!last) {
			const old = btreeInsert(m, item[0], item[1], k);
			if (old !== undefined) v?.(old);
			return;
		}
		const l = leafOf(m, item[0]);
		const i = search(l.k, item[0], m.c);
		if (i < 0) insertAt(m, l, ~i, item[0], item[1]);
		else {
			l.k[i] = item[0];
			l.v[i] = item[1];
		}
	};
	return extending(m, items, add, take);
}

/** `extend` of a set with keys, and with `last`, `collect` and `from`, as `btreeExtend`. */
export function btreeExtendKeys<K>(m: BTreeSet<K>, keys: Iter<K>, last = false, k?: (key: K) => void, take?: Take): BTreeSet<K> {
	const add = (key: K) => {
		if (!last) {
			btreeAdd(m, key, k);
			return;
		}
		const l = leafOf(m, key);
		const i = search(l.k, key, m.c);
		if (i < 0) insertAt(m, l, ~i, key, undefined);
		else l.k[i] = key;
	};
	return extending(m, keys, add, take);
}

/**
 * `collect` and `from` into `m`, an empty map, of `entries`, which std's collects into a
 * `Vec` first: it sorts them by key, stably, and builds the tree of each but the last of
 * equal keys, which drop in order.
 */
export function btreeCollect<K, V>(m: BTreeMap<K, V>, entries: [K, V][], k?: (key: K) => void, v?: (value: V) => void): BTreeMap<K, V> {
	const sorted = entries.sort((a, b) => m.c(a[0], b[0]));
	const keys: K[] = [];
	const values: V[] = [];
	for (let i = 0; i < sorted.length; i++) {
		const [key, value] = sorted[i];
		if (i + 1 === sorted.length || m.c(key, sorted[i + 1][0]) !== 0) {
			keys.push(key);
			values.push(value);
			continue;
		}
		dropPair(key, value, k, v);
	}
	build(m, keys, values);
	return m;
}

/** `collect` and `from` into `m`, an empty set, of `keys`, as `btreeCollect`. */
export function btreeCollectKeys<K>(m: BTreeSet<K>, keys: K[], k?: (key: K) => void): BTreeSet<K> {
	return btreeCollect(
		m,
		keys.map((key) => [key, undefined]),
		k,
	);
}

/** `clone`, with each key and value through its `Clone`, or as itself without one. */
export function btreeClone<K, V>(m: BTreeMap<K, V>, key: (k: K) => K = itself, value: (v: V) => V = itself): BTreeMap<K, V> {
	const keys: K[] = [];
	const values: V[] = [];
	// Each key, then its value, as std's clones them.
	for (let l: Leaf<K, V> | undefined = end(m, false); l !== undefined; l = l.next) {
		for (let i = 0; i < l.k.length; i++) {
			keys.push(key(l.k[i]));
			values.push(value(l.v[i]));
		}
	}
	const out = btreeNew<K, V>(m.c);
	build(out, keys, values);
	out.r = m.n > 0;
	return out;
}

// Entries

/**
 * `hash_map::Entry` and `btree_map::Entry`, and their `OccupiedEntry` and `VacantEntry`:
 * the map, the key, and where `entry` found it. The map stays as it was until the entry is
 * consumed, so nothing compares or hashes the key again, as nothing does in Rust.
 */
export interface Entry<K, V> {
	readonly m: HashMap<K, V> | BTreeMap<K, V>;
	/** The key it was given, or the map's own if it is occupied. */
	readonly k: K;
	/** Whether it is occupied: whether the map held the key. */
	readonly o: boolean;
	/** A hashed key's slot if it is occupied, and its hash. */
	readonly s: Slot<K, V> | undefined;
	readonly h: number | undefined;
	/** A B-tree's leaf, and the key's index in it, or the complement of where it goes. */
	readonly l: Leaf<K, V> | undefined;
	readonly i: number;
}

/**
 * `entry`: `key` drops at once if the map holds an equal one. A hashed key is hashed into
 * an empty map too, as std's is.
 */
export function entry<K, V>(m: HashMap<K, V> | BTreeMap<K, V>, key: K, k?: (key: K) => void): Entry<K, V> {
	const e = locate(m, key);
	if (e.o) k?.(key);
	return e;
}

function locate<K, V>(m: HashMap<K, V> | BTreeMap<K, V>, key: K): Entry<K, V> {
	if (!isHash(m)) {
		const l = leafOf(m, key);
		const i = search(l.k, key, m.c);
		return { m, k: i < 0 ? key : l.k[i], o: i >= 0, s: undefined, h: undefined, l, i };
	}
	if (!hashed(m, key)) return { m, k: key, o: m.p.has(key), s: undefined, h: undefined, l: undefined, i: 0 };
	const h = (m.h as Hashing<K>).hash(key);
	const s = slot(m, key, h);
	return { m, k: s === undefined ? key : s.k, o: s !== undefined, s, h, l: undefined, i: 0 };
}

/** Insert `value` where `e`, a vacant entry, goes: its place. */
function fill<K, V>(e: Entry<K, V>, value: V): Place<V> {
	const m = e.m;
	if (isHash(m)) {
		if (e.h !== undefined) return add(m, e.k, value, e.h);
		m.p.set(e.k, value);
		return new AtKey(m.p, e.k);
	}
	const l = e.l as Leaf<K, V>;
	const i = ~e.i;
	insertAt(m, l, i, e.k, value);
	// A split keeps the leaf's first half and moves the rest to the next leaf.
	return i < l.k.length ? new AtIndex(l.v, i) : new AtIndex((l.next as Leaf<K, V>).v, i - l.k.length);
}

/** Drop `e`, an `Entry`: a vacant one holds the key it was given. */
export function entryDrop<K, V>(e: Entry<K, V>, k: (key: K) => void): void {
	if (!e.o) k(e.k);
}

function isHash<K, V>(m: HashMap<K, V> | BTreeMap<K, V>): m is HashMap<K, V> {
	return "p" in m;
}

/** `Entry::or_insert_with`: the value there, or else `f()`'s, inserted. */
export function entryOrInsertWith<K, V>(e: Entry<K, V>, f: (key: K) => V): V {
	if (e.o) return entryGet(e);
	const value = f(e.k);
	fill(e, value);
	return value;
}

/** `Entry::or_insert`: the value there, and `value` drops, or else `value`, inserted. */
export function entryOrInsert<K, V>(e: Entry<K, V>, value: V, v?: (value: V) => void): V {
	if (!e.o) {
		fill(e, value);
		return value;
	}
	v?.(value);
	return entryGet(e);
}

/** Whether `e` is `Entry::Occupied`: whether the map holds its key. */
export function entryOccupied<K, V>(e: Entry<K, V>): boolean {
	return e.o;
}

/** `Entry::and_modify` of a value JS copies: `f` of its place, if any. */
export function entryAndModifyMut<K, V>(e: Entry<K, V>, f: (value: Place<V>) => void): Entry<K, V> {
	if (e.o) f(entryGetMut(e));
	return e;
}

/** `Entry::and_modify`: `f` of the value there, if any. */
export function entryAndModify<K, V>(e: Entry<K, V>, f: (value: V) => void): Entry<K, V> {
	if (e.o) f(entryGet(e));
	return e;
}

/** `Entry::key` and `OccupiedEntry::key`: the key the map holds, or else the one looked up. */
export function entryKey<K, V>(e: Entry<K, V>): K {
	return e.k;
}

/** `OccupiedEntry::get` and `get_mut`. */
export function entryGet<K, V>(e: Entry<K, V>): V {
	if (e.s !== undefined) return e.s.v;
	if (e.l !== undefined) return e.l.v[e.i];
	return (e.m as HashMap<K, V>).p.get(e.k) as V;
}

/** `OccupiedEntry::get_mut` and `into_mut` of a value JS copies: its place. */
export function entryGetMut<K, V>(e: Entry<K, V>): Place<V> {
	if (e.s !== undefined) return e.s;
	if (e.l !== undefined) return new AtIndex(e.l.v, e.i);
	return new AtKey((e.m as HashMap<K, V>).p, e.k);
}

/**
 * `Entry::or_insert_with` and `or_insert_with_key` of a value JS copies, and
 * `or_default`: its place, after inserting `f`'s value if the map lacks the key.
 */
export function entryOrInsertWithMut<K, V>(e: Entry<K, V>, f: (key: K) => V): Place<V> {
	return e.o ? entryGetMut(e) : fill(e, f(e.k));
}

/** `Entry::or_insert` and `VacantEntry::insert` of a value JS copies: its place, and `value` drops if one was there. */
export function entryOrInsertMut<K, V>(e: Entry<K, V>, value: V, v?: (value: V) => void): Place<V> {
	if (!e.o) return fill(e, value);
	const place = entryGetMut(e);
	v?.(value);
	return place;
}

/** `OccupiedEntry::insert`: the old value. */
export function entryInsert<K, V>(e: Entry<K, V>, value: V): V {
	const place = entryGetMut(e);
	const old = place.v;
	place.v = value;
	return old;
}

/** `OccupiedEntry::remove_entry`. */
export function entryRemoveEntry<K, V>(e: Entry<K, V>): [K, V] {
	const m = e.m;
	const value = entryGet(e);
	if (!isHash(m)) removeAt(m, e.l as Leaf<K, V>, e.i);
	else if (e.s !== undefined) unlink(m, e.h as number, e.s);
	else m.p.delete(e.k);
	return [e.k, value];
}

/** `OccupiedEntry::remove`, whose key drops. */
export function entryRemove<K, V>(e: Entry<K, V>, k?: (key: K) => void): V {
	const [key, value] = entryRemoveEntry(e);
	k?.(key);
	return value;
}

/** `first_entry`, and with `last`, `last_entry`: an occupied entry, if the map has one. */
export function btreeFirstEntry<K, V>(m: BTreeMap<K, V>, last = false): Entry<K, V> | undefined {
	if (m.n === 0) return undefined;
	const l = end(m, last);
	const i = last ? l.k.length - 1 : 0;
	return { m, k: l.k[i], o: true, s: undefined, h: undefined, l, i };
}

// VecDeque

/**
 * `VecDeque<T>`: a ring buffer of `b.length` slots, a power of two, holding `n` items
 * from `h` on, wrapping past the end.
 */
export interface VecDeque<T> {
	b: (T | undefined)[];
	h: number;
	n: number;
}

/** `VecDeque::new` and `with_capacity`. */
export function dequeNew<T>(): VecDeque<T> {
	return { b: [], h: 0, n: 0 };
}

/** The slot of item `i`. */
function at<T>(q: VecDeque<T>, i: number): number {
	return (q.h + i) & (q.b.length - 1);
}

/** Room for one more item. */
function grow<T>(q: VecDeque<T>): void {
	if (q.n < q.b.length) return;
	const b = new Array(Math.max(4, q.b.length * 2));
	for (let i = 0; i < q.n; i++) b[i] = q.b[at(q, i)];
	q.b = b;
	q.h = 0;
}

export function dequePushBack<T>(q: VecDeque<T>, item: T): void {
	grow(q);
	q.b[at(q, q.n++)] = item;
}

export function dequePushFront<T>(q: VecDeque<T>, item: T): void {
	grow(q);
	q.h = (q.h - 1) & (q.b.length - 1);
	q.b[q.h] = item;
	q.n++;
}

export function dequePopFront<T>(q: VecDeque<T>): T | undefined {
	if (q.n === 0) return undefined;
	const item = q.b[q.h];
	q.b[q.h] = undefined;
	q.h = at(q, 1);
	q.n--;
	return item;
}

export function dequePopBack<T>(q: VecDeque<T>): T | undefined {
	if (q.n === 0) return undefined;
	const i = at(q, --q.n);
	const item = q.b[i];
	q.b[i] = undefined;
	return item;
}

/** `get` and `get_mut`. */
export function dequeGet<T>(q: VecDeque<T>, i: number): T | undefined {
	return i < q.n ? q.b[at(q, i)] : undefined;
}

/** `front`, and with `back`, `back`. */
export function dequeEnd<T>(q: VecDeque<T>, back = false): T | undefined {
	return q.n === 0 ? undefined : q.b[at(q, back ? q.n - 1 : 0)];
}

/** `get_mut` of an item JS copies: its place, or `undefined` past the end. */
export function dequeGetMut<T>(q: VecDeque<T>, i: number): Place<T> | undefined {
	return i < q.n ? new AtIndex(q.b as T[], at(q, i)) : undefined;
}

/** `front_mut` of an item JS copies, and with `back`, `back_mut`. */
export function dequeEndMut<T>(q: VecDeque<T>, back = false): Place<T> | undefined {
	return q.n === 0 ? undefined : new AtIndex(q.b as T[], at(q, back ? q.n - 1 : 0));
}

/** `index` and `index_mut`: `q[i]`. */
export function dequeIndex<T>(q: VecDeque<T>, i: number): T {
	if (i >= q.n) panic("out of bounds access");
	return q.b[at(q, i)] as T;
}

/** `q[i] = item`, through `index_mut`. */
export function dequeSet<T>(q: VecDeque<T>, i: number, item: T): void {
	if (i >= q.n) panic("out of bounds access");
	q.b[at(q, i)] = item;
}

/** Drop `q`: each item by `f`, front to back. */
export function dequeDrop<T>(q: VecDeque<T>, f: (item: T) => void): void {
	const items: T[] = [];
	for (let i = 0; i < q.n; i++) items.push(q.b[at(q, i)] as T);
	dropAll(items, f);
}

/** `clear`, which leaves the deque empty and then drops each item, front to back. */
export function dequeClear<T>(q: VecDeque<T>, f?: (item: T) => void): void {
	const old: VecDeque<T> = { b: q.b, h: q.h, n: q.n };
	q.b = [];
	q.h = 0;
	q.n = 0;
	if (f !== undefined) dequeDrop(old, f);
}

/** `truncate`: the first `len` items, if there are more, and then each other drops in order. */
export function dequeTruncate<T>(q: VecDeque<T>, len: number, f?: (item: T) => void): void {
	if (f === undefined || q.n <= len) {
		while (q.n > len) dequePopBack(q);
		return;
	}
	const cut: T[] = [];
	for (let i = len; i < q.n; i++) cut.push(q.b[at(q, i)] as T);
	while (q.n > len) dequePopBack(q);
	dropAll(cut, f);
}

/**
 * `retain`, which visits the items in order and keeps each `f` returns true for. It swaps
 * each kept item into place, as std's does.
 */
export function dequeRetain<T>(q: VecDeque<T>, f: (item: T) => boolean, drop?: (item: T) => void): void {
	retainDeque(q, f as (item: T | Place<T>) => boolean, false, drop);
}

/** `retain_mut` of items JS copies: `f` takes the place of each. */
export function dequeRetainMut<T>(q: VecDeque<T>, f: (item: Place<T>) => boolean, drop?: (item: T) => void): void {
	retainDeque(q, f as (item: T | Place<T>) => boolean, true, drop);
}

/** `retain`, handing `f` each item or, if `lend`, its place. */
function retainDeque<T>(q: VecDeque<T>, f: (item: T | Place<T>) => boolean, lend: boolean, drop?: (item: T) => void): void {
	let kept = 0;
	for (let i = 0; i < q.n; i++) {
		const keep = f(lend ? new AtIndex(q.b as T[], at(q, i)) : (q.b[at(q, i)] as T));
		const item = q.b[at(q, i)] as T;
		if (!keep) continue;
		q.b[at(q, i)] = q.b[at(q, kept)];
		q.b[at(q, kept++)] = item;
	}
	dequeTruncate(q, kept, drop);
}

/** `contains`, by `eq`. */
export function dequeContains<T>(q: VecDeque<T>, item: T, eq: (a: T, b: T) => boolean): boolean {
	for (let i = 0; i < q.n; i++) if (eq(q.b[at(q, i)] as T, item)) return true;
	return false;
}

/** `remove`: the item at `i`, if any, with those after it moved up. */
export function dequeRemove<T>(q: VecDeque<T>, i: number): T | undefined {
	if (i >= q.n) return undefined;
	const item = q.b[at(q, i)];
	for (let j = i; j + 1 < q.n; j++) q.b[at(q, j)] = q.b[at(q, j + 1)];
	dequePopBack(q);
	return item;
}

/** `insert`: `item` at `i`, with those from `i` on moved down. */
export function dequeInsert<T>(q: VecDeque<T>, i: number, item: T): void {
	if (i > q.n) panic("index out of bounds");
	dequePushBack(q, item);
	for (let j = q.n - 1; j > i; j--) q.b[at(q, j)] = q.b[at(q, j - 1)];
	q.b[at(q, i)] = item;
}

/**
 * An iterator of a deque's items from index `front` up to `back`, which drops those it
 * still holds by `drop` as it drops, if it owns them, as a `drain` does.
 */
class DequeIter<T> extends Iter<T> implements Disposable {
	private readonly q: VecDeque<T>;
	private front: number;
	private back: number;
	private readonly drop: ((item: T) => void) | undefined;

	constructor(q: VecDeque<T>, front: number, back: number, drop?: (item: T) => void) {
		super();
		this.q = q;
		this.front = front;
		this.back = back;
		this.drop = drop;
	}

	next(): T | Done {
		return this.front < this.back ? (this.q.b[at(this.q, this.front++)] as T) : DONE;
	}

	override nextBack(): T | Done {
		return this.front < this.back ? (this.q.b[at(this.q, --this.back)] as T) : DONE;
	}

	override len(): number {
		return this.back - this.front;
	}

	[Symbol.dispose](): void {
		if (this.drop === undefined) return;
		const items: T[] = [];
		while (this.front < this.back) items.push(this.q.b[at(this.q, this.front++)] as T);
		dropAll(items, this.drop);
	}
}

/**
 * `iter` and `iter_mut`, and with what drops each item, `into_iter`, whose `last`,
 * `count`, and skips are a `Vec`'s.
 */
export function dequeIter<T>(q: VecDeque<T>, drop?: (item: T) => void): Iter<T> {
	if (drop === undefined) return new DequeIter(q, 0, q.n);
	const items: T[] = [];
	for (let i = 0; i < q.n; i++) items.push(q.b[at(q, i)] as T);
	return iterSlice(items, drop);
}

/** `iter_mut` of items JS copies: the place of each. */
export function dequeIterMut<T>(q: VecDeque<T>): Iter<Place<T>> {
	return new DequePlaces(q);
}

/** A deque's `iter_mut` of items JS copies, which lends each as its place. */
class DequePlaces<T> extends Iter<Place<T>> {
	private readonly q: VecDeque<T>;
	private front = 0;
	private back: number;

	constructor(q: VecDeque<T>) {
		super();
		this.q = q;
		this.back = q.n;
	}

	next(): Place<T> | Done {
		return this.front < this.back ? new AtIndex(this.q.b as T[], at(this.q, this.front++)) : DONE;
	}

	override nextBack(): Place<T> | Done {
		return this.front < this.back ? new AtIndex(this.q.b as T[], at(this.q, --this.back)) : DONE;
	}

	override len(): number {
		return this.back - this.front;
	}
}

/** `drain`: the items within `r`, taken out at once, which drop by `drop` as `dequeIter`'s. */
export function dequeDrain<T>(q: VecDeque<T>, r: Bounds<number>, drop?: (item: T) => void): Iter<T> {
	// As `slice::range` checks it.
	const [lo, hi, inclusive] = bounds(r);
	if (hi !== undefined && (inclusive ? hi.key >= q.n : hi.key > q.n)) panic(`range end index ${hi.key} out of range for slice of length ${q.n}`);
	const end = hi === undefined ? q.n : inclusive ? hi.key + 1 : hi.key;
	const start = lo === undefined ? 0 : lo.key;
	if (start > end) {
		panic(start > q.n ? `range start index ${start} out of range for slice of length ${q.n}` : `slice index starts at ${start} but ends at ${end}`);
	}
	const taken: VecDeque<T> = { b: [], h: 0, n: 0 };
	for (let i = start; i < end; i++) dequePushBack(taken, q.b[at(q, i)] as T);
	for (let i = end; i < q.n; i++) q.b[at(q, i - (end - start))] = q.b[at(q, i)];
	dequeTruncate(q, q.n - (end - start));
	return new DequeIter(taken, 0, taken.n, drop);
}

/** `extend`, and `collect` into a deque through `dequeNew`, which takes `items` as `take` says. */
export function dequeExtend<T>(q: VecDeque<T>, items: Iter<T>, take?: Take): VecDeque<T> {
	return extending(q, items, (item) => dequePushBack(q, item), take);
}

/** `clone`, with each item through its `Clone`, or as itself without one. */
export function dequeClone<T>(q: VecDeque<T>, item: (value: T) => T = itself): VecDeque<T> {
	const out = dequeNew<T>();
	for (let i = 0; i < q.n; i++) dequePushBack(out, item(q.b[at(q, i)] as T));
	return out;
}
