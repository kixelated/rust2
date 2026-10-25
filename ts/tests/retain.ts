// `retain` whose predicate panics, caught as a caller of a translation may catch it:
// over every keep mask of up to 6 items and each item to panic on, prints what a
// B-tree and a deque hold after, which `runtime.rs` compares with std's. A hash map's
// visiting order is unspecified, so it is checked here instead: whole, holding each
// entry but the ones `f` rejected before it panicked.
//
// usage: node retain.ts

import * as rt from "../runtime/rt.ts";

class Thrown extends Error {}

for (let n = 0; n <= 6; n++) {
	for (let mask = 0; mask < 1 << n; mask++) {
		for (let p = 0; p <= n; p++) {
			const keep = (i: number) => {
				if (i === p) throw new Thrown();
				return ((mask >> i) & 1) === 1;
			};
			const caught = (run: () => void) => {
				try {
					run();
				} catch (err) {
					if (!(err instanceof Thrown)) throw err;
				}
			};

			const tree = rt.btreeNew<number, number>(rt.cmp);
			for (let i = n - 1; i >= 0; i--) rt.btreeInsert(tree, i, i * 10);
			caught(() => rt.btreeRetain(tree, (k) => keep(k)));

			// Half pushed at the front, so the ring wraps.
			const deque = rt.dequeNew<number>();
			for (let i = (n >> 1) - 1; i >= 0; i--) rt.dequePushFront(deque, i);
			for (let i = n >> 1; i < n; i++) rt.dequePushBack(deque, i);
			caught(() => rt.dequeRetain(deque, keep));

			const entries = rt.btreeIter(tree).toArray();
			const items = rt.dequeIter(deque).toArray();
			console.log(`${n} ${mask} ${p} btree ${entries.join(";")} deque ${items.join(",")}`);

			// Keys that collide by twos, so `retain` unlinks within chains.
			type Key = { k: number };
			const hashed = rt.hashNew<Key, number>(
				rt.hashing<Key>(
					(key, s) => rt.hashInt(key.k >> 1, s),
					(a, b) => a.k === b.k,
				),
			);
			for (let i = 0; i < n; i++) rt.hashInsert(hashed, { k: i }, i);
			const rejected = new Set<number>();
			caught(() =>
				rt.hashRetain(hashed, (key) => {
					const kept = keep(key.k);
					if (!kept) rejected.add(key.k);
					return kept;
				}),
			);
			const held = rt.hashIter(hashed).toArray().map(([key, v]) => {
				if (rt.hashGet(hashed, key) !== v) throw new Error(`${key.k} is not found`);
				return key.k;
			});
			const want = [...Array(n).keys()].filter((i) => !rejected.has(i));
			if (rt.hashLen(hashed) !== held.length || held.sort().join() !== want.join()) {
				throw new Error(`hash map of ${n} by ${mask} panicking at ${p}: ${held} of ${rt.hashLen(hashed)}`);
			}
		}
	}
}
