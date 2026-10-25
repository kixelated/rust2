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

import { lt, type Limbs, panic } from "../rt.ts";
import { DONE, type Done, Iter, RangeInclusive } from "./iter.ts";

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

/** The slot holding a key equal to `key`, a hashed key. */
function slot<K, V>(m: HashMap<K, V>, key: K): Slot<K, V> | undefined {
	const h = m.h as Hashing<K>;
	for (let s = m.o?.get(h.hash(key)); s !== undefined; s = s.next) if (h.eq(s.k, key)) return s;
	return undefined;
}

export function hashLen<K, V>(m: HashMap<K, V>): number {
	return m.p.size + m.n;
}

export function hashGet<K, V>(m: HashMap<K, V>, key: K): V | undefined {
	return hashed(m, key) ? slot(m, key)?.v : m.p.get(key);
}

export function hashContains<K, V>(m: HashMap<K, V>, key: K): boolean {
	return hashed(m, key) ? slot(m, key) !== undefined : m.p.has(key);
}

/** `HashMap::index`: `map[&key]`. */
export function hashIndex<K, V>(m: HashMap<K, V>, key: K): V {
	if (!hashContains(m, key)) panic("no entry found for key");
	return hashGet(m, key) as V;
}

/** `HashMap::insert`: the old value, if any. An equal key already there stays. */
export function hashInsert<K, V>(m: HashMap<K, V>, key: K, value: V): V | undefined {
	if (!hashed(m, key)) {
		const old = m.p.get(key);
		m.p.set(key, value);
		return old;
	}
	const found = slot(m, key);
	if (found !== undefined) {
		const old = found.v;
		found.v = value;
		return old;
	}
	add(m, key, value);
	return undefined;
}

/** Add a hashed key that is not there yet. */
function add<K, V>(m: HashMap<K, V>, key: K, value: V): void {
	m.o ??= new Map();
	const hash = (m.h as Hashing<K>).hash(key);
	m.o.set(hash, { k: key, v: value, next: m.o.get(hash) });
	m.n++;
}

/** `HashSet::insert`: whether `key` was not there yet. An equal key already there stays. */
export function hashAdd<K>(m: HashSet<K>, key: K): boolean {
	if (hashContains(m, key)) return false;
	if (hashed(m, key)) add(m, key, undefined);
	else m.p.set(key, undefined);
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
	const h = m.h as Hashing<K>;
	const hash = h.hash(key);
	let prev: Slot<K, V> | undefined;
	for (let s = m.o?.get(hash); s !== undefined; prev = s, s = s.next) {
		if (!h.eq(s.k, key)) continue;
		if (prev !== undefined) prev.next = s.next;
		else if (s.next !== undefined) m.o?.set(hash, s.next);
		else m.o?.delete(hash);
		m.n--;
		return [s.k, s.v];
	}
	return undefined;
}

/** `HashMap::remove`: the value there, if any. */
export function hashRemove<K, V>(m: HashMap<K, V>, key: K): V | undefined {
	if (!hashed(m, key)) {
		const old = m.p.get(key);
		m.p.delete(key);
		return old;
	}
	return hashRemoveEntry(m, key)?.[1];
}

/** `HashSet::remove`: whether `key` was there. */
export function hashDelete<K>(m: HashSet<K>, key: K): boolean {
	return hashed(m, key) ? hashRemoveEntry(m, key) !== undefined : m.p.delete(key);
}

/** `HashSet::get`, and `HashSet::take` if `take`: the equal key the set holds. */
export function hashKey<K>(m: HashSet<K>, key: K, take = false): K | undefined {
	if (hashed(m, key)) return take ? hashRemoveEntry(m, key)?.[0] : slot(m, key)?.k;
	if (!m.p.has(key)) return undefined;
	if (take) m.p.delete(key);
	return key;
}

export function hashClear<K, V>(m: HashMap<K, V>): void {
	m.p.clear();
	m.o = undefined;
	m.n = 0;
}

/** `retain`, which keeps each entry `f` returns true for. */
export function hashRetain<K, V>(m: HashMap<K, V>, f: (key: K, value: V) => boolean): void {
	for (const [k, v] of m.p) if (!f(k, v)) m.p.delete(k);
	if (m.o === undefined) return;
	for (const [hash, first] of m.o) {
		let head: Slot<K, V> | undefined;
		let tail: Slot<K, V> | undefined;
		for (let s: Slot<K, V> | undefined = first; s !== undefined; s = s.next) {
			if (!f(s.k, s.v)) {
				m.n--;
				continue;
			}
			if (tail === undefined) head = s;
			else tail.next = s;
			tail = s;
		}
		if (tail === undefined) m.o.delete(hash);
		else {
			tail.next = undefined;
			if (head !== first) m.o.set(hash, head as Slot<K, V>);
		}
	}
}

/** What a collection's iterator yields: its entries as `[key, value]`, keys, or values. */
type Pick = 0 | 1 | 2;
const ENTRIES = 0;
const KEYS = 1;
const VALUES = 2;

/** An iterator of a hash collection: `iter`, `keys`, `values`, `drain`, and the rest. */
class HashIter<K, V, T> extends Iter<T> {
	private readonly p: Iterator<[K, V]>;
	private readonly o: Iterator<Slot<K, V>> | undefined;
	private at: Slot<K, V> | undefined;
	private readonly pick: Pick;
	private left: number;

	constructor(m: HashMap<K, V>, pick: Pick) {
		super();
		this.p = m.p.entries();
		this.o = m.o?.values();
		this.pick = pick;
		this.left = hashLen(m);
	}

	next(): T | Done {
		if (this.left === 0) return DONE;
		this.left--;
		const entry = this.p.next();
		if (!entry.done) {
			const [k, v] = entry.value;
			return (this.pick === ENTRIES ? entry.value : this.pick === KEYS ? k : v) as T;
		}
		if (this.at === undefined) this.at = (this.o as Iterator<Slot<K, V>>).next().value;
		const s = this.at as Slot<K, V>;
		this.at = s.next;
		return (this.pick === ENTRIES ? [s.k, s.v] : this.pick === KEYS ? s.k : s.v) as T;
	}

	override len(): number {
		return this.left;
	}
}

/** `iter`, `iter_mut`, and `into_iter` of a map: its entries, as `[key, value]`. */
export function hashIter<K, V>(m: HashMap<K, V>): Iter<[K, V]> {
	return new HashIter<K, V, [K, V]>(m, ENTRIES);
}

/** `keys` and `into_keys` of a map, and `iter` and `into_iter` of a set. */
export function hashKeys<K, V>(m: HashMap<K, V>): Iter<K> {
	return new HashIter<K, V, K>(m, KEYS);
}

/** `values`, `values_mut`, and `into_values`. */
export function hashValues<K, V>(m: HashMap<K, V>): Iter<V> {
	return new HashIter<K, V, V>(m, VALUES);
}

/** `drain` of a map: every entry, taken out at once. */
export function hashDrain<K, V>(m: HashMap<K, V>): Iter<[K, V]> {
	return hashIter(taken(m));
}

/** `drain` of a set: every key, taken out at once. */
export function hashDrainKeys<K, V>(m: HashMap<K, V>): Iter<K> {
	return hashKeys(taken(m));
}

/** What `m` held, which it no longer does. */
function taken<K, V>(m: HashMap<K, V>): HashMap<K, V> {
	const out: HashMap<K, V> = { p: m.p, o: m.o, n: m.n, h: m.h };
	m.p = new Map();
	hashClear(m);
	return out;
}

/** `extend` of a map with `[key, value]`s, and `collect` into one through `hashNew`. */
export function hashExtend<K, V>(m: HashMap<K, V>, items: Iter<[K, V]>): HashMap<K, V> {
	for (let item = items.next(); item !== DONE; item = items.next()) hashInsert(m, item[0], item[1]);
	return m;
}

/** `extend` of a set with keys, and `collect` into one through `hashNew`. */
export function hashExtendKeys<K>(m: HashSet<K>, keys: Iter<K>): HashSet<K> {
	for (let key = keys.next(); key !== DONE; key = keys.next()) hashAdd(m, key);
	return m;
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
 * `BTreeMap<K, V>`, and `BTreeSet<K>` as a map to `undefined`: its keys sorted by their
 * `Ord`, and each key's value at the same index.
 */
export interface BTreeMap<K, V> {
	k: K[];
	v: V[];
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

/** `BTreeMap::new` and `BTreeSet::new`, ordered by `cmp`. */
export function btreeNew<K, V>(cmp: (a: K, b: K) => Ordering): BTreeMap<K, V> {
	return { k: [], v: [], c: cmp, r: false };
}

/** The index of `key` if the map has it, or else the bitwise not of where it would go. */
function search<K, V>(m: BTreeMap<K, V>, key: K): number {
	let lo = 0;
	let hi = m.k.length;
	while (lo < hi) {
		const mid = (lo + hi) >>> 1;
		const c = m.c(m.k[mid], key);
		if (c === 0) return mid;
		if (c < 0) lo = mid + 1;
		else hi = mid;
	}
	return ~lo;
}

export function btreeLen<K, V>(m: BTreeMap<K, V>): number {
	return m.k.length;
}

export function btreeGet<K, V>(m: BTreeMap<K, V>, key: K): V | undefined {
	const i = search(m, key);
	return i < 0 ? undefined : m.v[i];
}

/** `BTreeMap::get_key_value`, and `BTreeSet::get`, which return the equal key the map holds. */
export function btreeGetKeyValue<K, V>(m: BTreeMap<K, V>, key: K): [K, V] | undefined {
	const i = search(m, key);
	return i < 0 ? undefined : [m.k[i], m.v[i]];
}

export function btreeContains<K, V>(m: BTreeMap<K, V>, key: K): boolean {
	return search(m, key) >= 0;
}

/** `BTreeMap::index`: `map[&key]`. */
export function btreeIndex<K, V>(m: BTreeMap<K, V>, key: K): V {
	const i = search(m, key);
	if (i < 0) panic("no entry found for key");
	return m.v[i];
}

/** `BTreeMap::insert`: the old value, if any. An equal key already there stays. */
export function btreeInsert<K, V>(m: BTreeMap<K, V>, key: K, value: V): V | undefined {
	const i = search(m, key);
	if (i >= 0) {
		const old = m.v[i];
		m.v[i] = value;
		return old;
	}
	insertAt(m, ~i, key, value);
	return undefined;
}

function insertAt<K, V>(m: BTreeMap<K, V>, i: number, key: K, value: V): void {
	m.r = true;
	if (i === m.k.length) {
		m.k.push(key);
		m.v.push(value);
	} else {
		m.k.splice(i, 0, key);
		m.v.splice(i, 0, value);
	}
}

/** `BTreeSet::insert`: whether `key` was not there yet. An equal key already there stays. */
export function btreeAdd<K>(m: BTreeSet<K>, key: K): boolean {
	const i = search(m, key);
	if (i >= 0) return false;
	insertAt(m, ~i, key, undefined);
	return true;
}

/** The key and value at `i`, taken out. */
function removeAt<K, V>(m: BTreeMap<K, V>, i: number): [K, V] {
	if (i === 0) return [m.k.shift() as K, m.v.shift() as V];
	if (i === m.k.length - 1) return [m.k.pop() as K, m.v.pop() as V];
	return [m.k.splice(i, 1)[0], m.v.splice(i, 1)[0]];
}

/** `BTreeMap::remove_entry`, and `BTreeSet::take`. */
export function btreeRemoveEntry<K, V>(m: BTreeMap<K, V>, key: K): [K, V] | undefined {
	const i = search(m, key);
	return i < 0 ? undefined : removeAt(m, i);
}

export function btreeRemove<K, V>(m: BTreeMap<K, V>, key: K): V | undefined {
	return btreeRemoveEntry(m, key)?.[1];
}

/** `BTreeSet::remove`: whether `key` was there. */
export function btreeDelete<K>(m: BTreeSet<K>, key: K): boolean {
	return btreeRemoveEntry(m, key) !== undefined;
}

/** `first_key_value`, and with `last`, `last_key_value`. */
export function btreeFirst<K, V>(m: BTreeMap<K, V>, last = false): [K, V] | undefined {
	const n = m.k.length;
	if (n === 0) return undefined;
	const i = last ? n - 1 : 0;
	return [m.k[i], m.v[i]];
}

/** `BTreeSet::first`, and with `last`, `BTreeSet::last`. */
export function btreeFirstKey<K, V>(m: BTreeMap<K, V>, last = false): K | undefined {
	return last ? m.k[m.k.length - 1] : m.k[0];
}

/** `pop_first`, and with `last`, `pop_last`. */
export function btreePop<K, V>(m: BTreeMap<K, V>, last = false): [K, V] | undefined {
	const n = m.k.length;
	return n === 0 ? undefined : removeAt(m, last ? n - 1 : 0);
}

/** `BTreeSet::pop_first`, and with `last`, `pop_last`. */
export function btreePopKey<K, V>(m: BTreeMap<K, V>, last = false): K | undefined {
	return btreePop(m, last)?.[0];
}

export function btreeClear<K, V>(m: BTreeMap<K, V>): void {
	m.k = [];
	m.v = [];
	m.r = false;
}

/** `retain`, which visits the keys in order and keeps each entry `f` returns true for. */
export function btreeRetain<K, V>(m: BTreeMap<K, V>, f: (key: K, value: V) => boolean): void {
	let kept = 0;
	for (let i = 0; i < m.k.length; i++) {
		if (!f(m.k[i], m.v[i])) continue;
		m.k[kept] = m.k[i];
		m.v[kept] = m.v[i];
		kept++;
	}
	m.k.length = kept;
	m.v.length = kept;
}

/** An iterator of a B-tree's entries, keys, or values from index `front` up to `back`. */
class BTreeIter<K, V, T> extends Iter<T> {
	private readonly k: K[];
	private readonly v: V[];
	private front: number;
	private back: number;
	private readonly pick: Pick;

	constructor(m: BTreeMap<K, V>, pick: Pick, front = 0, back = m.k.length) {
		super();
		this.k = m.k;
		this.v = m.v;
		this.pick = pick;
		this.front = front;
		this.back = back;
	}

	private at(i: number): T {
		return (this.pick === ENTRIES ? [this.k[i], this.v[i]] : this.pick === KEYS ? this.k[i] : this.v[i]) as T;
	}

	next(): T | Done {
		return this.front < this.back ? this.at(this.front++) : DONE;
	}

	override nextBack(): T | Done {
		return this.front < this.back ? this.at(--this.back) : DONE;
	}

	override len(): number {
		return this.back - this.front;
	}
}

/** `iter`, `iter_mut`, and `into_iter` of a map: its entries in key order. */
export function btreeIter<K, V>(m: BTreeMap<K, V>): Iter<[K, V]> {
	return new BTreeIter<K, V, [K, V]>(m, ENTRIES);
}

/** `keys` and `into_keys` of a map, and `iter` and `into_iter` of a set. */
export function btreeKeys<K, V>(m: BTreeMap<K, V>): Iter<K> {
	return new BTreeIter<K, V, K>(m, KEYS);
}

/** `values`, `values_mut`, and `into_values`. */
export function btreeValues<K, V>(m: BTreeMap<K, V>): Iter<V> {
	return new BTreeIter<K, V, V>(m, VALUES);
}

/**
 * A range of keys or indices, as its value is: `a..b`, `a..=b`, `a..` as `{ start }`, `..b`
 * as `{ end }`, or `..` as `undefined`.
 */
export type Bounds<K> = { readonly start: K; readonly end: K } | RangeInclusive<K> | { readonly start: K } | { readonly end: K } | undefined;

/** `RangeBounds`: the start, the end, and whether the end is in the range. */
function bounds<K>(r: Bounds<K>): [{ key: K } | undefined, { key: K } | undefined, boolean] {
	if (r === undefined) return [undefined, undefined, false];
	// Exhausted, an inclusive range ends before its end, as its `end_bound` says.
	if (r instanceof RangeInclusive) return [{ key: r.start() }, { key: r.end() }, r.endIncluded()];
	return ["start" in r ? { key: r.start } : undefined, "end" in r ? { key: r.end } : undefined, false];
}

/** `BTreeMap::range` and `range_mut`: the entries within `r`. */
export function btreeRange<K, V>(m: BTreeMap<K, V>, r: Bounds<K>): Iter<[K, V]> {
	const [front, back] = span(m, r, "BTreeMap");
	return new BTreeIter<K, V, [K, V]>(m, ENTRIES, front, back);
}

/** `BTreeSet::range`: the keys within `r`. */
export function btreeRangeKeys<K, V>(m: BTreeMap<K, V>, r: Bounds<K>): Iter<K> {
	const [front, back] = span(m, r, "BTreeSet");
	return new BTreeIter<K, V, K>(m, KEYS, front, back);
}

/** The indices from and up to which `r` runs in `m`, checked as std checks it. */
function span<K, V>(m: BTreeMap<K, V>, r: Bounds<K>, name: string): [number, number] {
	if (!m.r) return [0, 0];
	const [start, end, inclusive] = bounds(r);
	if (start !== undefined && end !== undefined && m.c(start.key, end.key) > 0) {
		panic(`range start is greater than range end in ${name}`);
	}
	const front = start === undefined ? 0 : edge(m, start.key, false);
	const back = end === undefined ? m.k.length : edge(m, end.key, inclusive);
	return [front, Math.max(front, back)];
}

/** The index of the first key past `key` if `past`, or else of the first key not below it. */
function edge<K, V>(m: BTreeMap<K, V>, key: K, past: boolean): number {
	const i = search(m, key);
	return i < 0 ? ~i : past ? i + 1 : i;
}

/**
 * `extend` of a map with `[key, value]`s, which keeps an equal key already there, and with
 * `last`, `collect` and `from` into an empty one, which keep the last of equal keys.
 */
export function btreeExtend<K, V>(m: BTreeMap<K, V>, items: Iter<[K, V]>, last = false): BTreeMap<K, V> {
	for (let item = items.next(); item !== DONE; item = items.next()) {
		const i = search(m, item[0]);
		if (i < 0) insertAt(m, ~i, item[0], item[1]);
		else {
			if (last) m.k[i] = item[0];
			m.v[i] = item[1];
		}
	}
	return m;
}

/** `extend` of a set with keys, and with `last`, `collect` and `from`, as `btreeExtend`. */
export function btreeExtendKeys<K>(m: BTreeSet<K>, keys: Iter<K>, last = false): BTreeSet<K> {
	for (let key = keys.next(); key !== DONE; key = keys.next()) {
		const i = search(m, key);
		if (i < 0) insertAt(m, ~i, key, undefined);
		else if (last) m.k[i] = key;
	}
	return m;
}

/** `clone`, with each key and value through its `Clone`, or as itself without one. */
export function btreeClone<K, V>(m: BTreeMap<K, V>, key: (k: K) => K = itself, value: (v: V) => V = itself): BTreeMap<K, V> {
	return { k: m.k.map(key), v: m.v.map(value), c: m.c, r: m.k.length > 0 };
}

// Entries

/**
 * `hash_map::Entry` and `btree_map::Entry`, and their `OccupiedEntry`: the map, and the
 * key looked up, which an occupied entry finds again.
 */
export interface Entry<K, V> {
	readonly m: HashMap<K, V> | BTreeMap<K, V>;
	readonly k: K;
}

/** `entry`. */
export function entry<K, V>(m: HashMap<K, V> | BTreeMap<K, V>, key: K): Entry<K, V> {
	return { m, k: key };
}

function isHash<K, V>(m: HashMap<K, V> | BTreeMap<K, V>): m is HashMap<K, V> {
	return "p" in m;
}

/** `Entry::or_insert_with`: the value there, or else `make()`'s, inserted. */
export function entryOrInsertWith<K, V>(e: Entry<K, V>, make: (key: K) => V): V {
	const m = e.m;
	if (isHash(m)) {
		const found = hashGet(m, e.k);
		if (found !== undefined) return found;
		const value = make(e.k);
		hashInsert(m, e.k, value);
		return value;
	}
	const i = search(m, e.k);
	if (i >= 0) return m.v[i];
	const value = make(e.k);
	insertAt(m, ~i, e.k, value);
	return value;
}

/** `Entry::or_insert`: the value there, or else `value`, inserted. */
export function entryOrInsert<K, V>(e: Entry<K, V>, value: V): V {
	return entryOrInsertWith(e, () => value);
}

/** Whether `e` is `Entry::Occupied`: whether the map holds its key. */
export function entryOccupied<K, V>(e: Entry<K, V>): boolean {
	return isHash(e.m) ? hashContains(e.m, e.k) : btreeContains(e.m, e.k);
}

/** `Entry::and_modify`: `f` of the value there, if any, which is never `undefined`. */
export function entryAndModify<K, V>(e: Entry<K, V>, f: (value: V) => void): Entry<K, V> {
	const value = isHash(e.m) ? hashGet(e.m, e.k) : btreeGet(e.m, e.k);
	if (value !== undefined) f(value);
	return e;
}

/** `Entry::key` and `OccupiedEntry::key`: the key the map holds, or else the one looked up. */
export function entryKey<K, V>(e: Entry<K, V>): K {
	if (isHash(e.m)) return hashed(e.m, e.k) ? (slot(e.m, e.k)?.k ?? e.k) : e.k;
	const i = search(e.m, e.k);
	return i < 0 ? e.k : e.m.k[i];
}

/** `OccupiedEntry::get` and `get_mut`. */
export function entryGet<K, V>(e: Entry<K, V>): V {
	return (isHash(e.m) ? hashGet(e.m, e.k) : btreeGet(e.m, e.k)) as V;
}

/** `OccupiedEntry::insert`: the old value. */
export function entryInsert<K, V>(e: Entry<K, V>, value: V): V {
	return (isHash(e.m) ? hashInsert(e.m, e.k, value) : btreeInsert(e.m, e.k, value)) as V;
}

/** `OccupiedEntry::remove_entry`. */
export function entryRemoveEntry<K, V>(e: Entry<K, V>): [K, V] {
	return (isHash(e.m) ? hashRemoveEntry(e.m, e.k) : btreeRemoveEntry(e.m, e.k)) as [K, V];
}

/** `OccupiedEntry::remove`. */
export function entryRemove<K, V>(e: Entry<K, V>): V {
	return entryRemoveEntry(e)[1];
}

/** `first_entry`, and with `last`, `last_entry`: an occupied entry, if the map has one. */
export function btreeFirstEntry<K, V>(m: BTreeMap<K, V>, last = false): Entry<K, V> | undefined {
	const key = btreeFirstKey(m, last);
	return m.k.length === 0 ? undefined : { m, k: key as K };
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

export function dequeClear<T>(q: VecDeque<T>): void {
	q.b = [];
	q.h = 0;
	q.n = 0;
}

/** `truncate`: the first `len` items, if there are more. */
export function dequeTruncate<T>(q: VecDeque<T>, len: number): void {
	while (q.n > len) dequePopBack(q);
}

/** `retain`, which visits the items in order and keeps each `f` returns true for. */
export function dequeRetain<T>(q: VecDeque<T>, f: (item: T) => boolean): void {
	let kept = 0;
	for (let i = 0; i < q.n; i++) {
		const item = q.b[at(q, i)] as T;
		if (!f(item)) continue;
		q.b[at(q, kept++)] = item;
	}
	dequeTruncate(q, kept);
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

/** An iterator of a deque's items from index `front` up to `back`. */
class DequeIter<T> extends Iter<T> {
	private readonly q: VecDeque<T>;
	private front: number;
	private back: number;

	constructor(q: VecDeque<T>, front: number, back: number) {
		super();
		this.q = q;
		this.front = front;
		this.back = back;
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
}

/** `iter`, `iter_mut`, and `into_iter`. */
export function dequeIter<T>(q: VecDeque<T>): Iter<T> {
	return new DequeIter(q, 0, q.n);
}

/** `drain`: the items within `r`, taken out at once. */
export function dequeDrain<T>(q: VecDeque<T>, r: Bounds<number>): Iter<T> {
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
	return new DequeIter(taken, 0, taken.n);
}

/** `extend`, and `collect` into a deque through `dequeNew`. */
export function dequeExtend<T>(q: VecDeque<T>, items: Iter<T>): VecDeque<T> {
	for (let item = items.next(); item !== DONE; item = items.next()) dequePushBack(q, item);
	return q;
}

/** `clone`, with each item through its `Clone`, or as itself without one. */
export function dequeClone<T>(q: VecDeque<T>, item: (value: T) => T = itself): VecDeque<T> {
	const out = dequeNew<T>();
	for (let i = 0; i < q.n; i++) dequePushBack(out, item(q.b[at(q, i)] as T));
	return out;
}
