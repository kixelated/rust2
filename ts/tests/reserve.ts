// A `BytesMut` whose storage was shared throws a rust2-ts error on a reserve past 512 MiB,
// counting what it read past, which native Rust can't show: there `bytes` panics with
// "overflow" or "capacity overflow", or grows, by whether other handles of it dropped.
//
// usage: node reserve.ts

import { strict as assert } from "node:assert";

import * as rt from "../runtime/rt.ts";

const LIMIT = 2 ** 29;
const unsure = (f: () => void) => assert.throws(f, (e: Error) => !(e instanceof rt.Panic) && e.message.startsWith("rust2-ts: "));
const from = (s: string) => rt.bytesMutFrom(rt.stringAsBytes(s));

// One past the limit, counting the two bytes read past.
const edge = from("edges");
rt.bytesMutAdvance(edge, 2);
const head = rt.bytesMutSplitTo(edge, 0);
unsure(() => rt.bytesMutReserve(edge, LIMIT + 1 - 2 - 3));
// Its sibling, from the same place, reaches it exactly.
rt.bytesMutReserve(head, LIMIT - 2);

// Uniquely held after the split, `bytes` panics with "overflow", and held, "capacity overflow".
const unique = from("ab");
rt.bytesMutSplitTo(unique, 1);
unsure(() => rt.bytesMutReserve(unique, 0xffffffff - 1));

// Grown, split again, and grown within what it holds here, `bytes` may have moved it into a vector.
const grown = from("x");
rt.bytesMutSplitTo(grown, 0);
rt.bytesMutReserve(grown, 1);
rt.bytesMutSplitTo(grown, 0);
rt.bytesMutReserve(grown, 2);
unsure(() => rt.bytesMutReserve(grown, 0xffffffff));

// Taken from a `Bytes`, the same.
unsure(() => rt.bytesMutReserve(rt.bytesMutFromBytes(rt.stringAsBytes("x")), LIMIT));

// A vector of its own panics as `Vec` does past `isize::MAX`.
assert.throws(
	() => rt.bytesMutReserve(from("x"), 0xffffffff),
	(e: Error) => e instanceof rt.Panic && e.message === "capacity overflow",
);
