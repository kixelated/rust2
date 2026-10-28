// A NaN's payload and sign, which JS leaves to the engine: what this engine keeps through
// `from_bits` and `to_bits`, the sign tests, and `copysign`, and that it quiets a
// signaling NaN stored in an array, as V8 does, which the README notes.
//
// usage: node nan.ts

import { strict as assert } from "node:assert";

import * as rt from "../runtime/rt.ts";

/** A `u64`'s value. */
function big(v: rt.u64): bigint {
	return typeof v === "number" ? BigInt(v) : (BigInt(v.hi) << 32n) + BigInt(v.lo);
}

const QUIET = 0x0008000000000000n;
for (const bits of [0x7ff8000000000000n, 0xfff8000000000000n, 0x7ff8000000000123n, 0xfffc0000deadbeefn, 0x7ff0000000000001n, 0xfff4000000000abcn]) {
	const x = rt.f64FromBits(rt.u64Of(bits));
	const negative = bits >> 63n === 1n;
	assert.equal(big(rt.f64ToBits(x)), bits, `${bits.toString(16)} round trips`);
	assert.equal(rt.f64IsSignNegative(x), negative);
	assert.equal(rt.f64IsSignPositive(x), !negative);
	assert.equal(big(rt.f64ToBits(rt.f64Copysign(x, -1))), bits | (1n << 63n));
	assert.equal(big(rt.f64ToBits(rt.f64Copysign(x, 1))), bits & ~(1n << 63n));
	// A double array holds a NaN quieted, its payload otherwise kept.
	const held = [x, 1.5];
	assert.equal(big(rt.f64ToBits(held[0])), bits | QUIET, `${bits.toString(16)} held in an array`);
}
