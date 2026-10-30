// `key::Nibble`, a byte that compares, hashes, and orders by its low four bits, which the
// `===` of its number would not.

import * as rt from "../rt.ts";

export type Nibble = rt.u8;

function make(byte: rt.u8): Nibble {
	return byte;
}
export { make as new };

export function get(self: Nibble): rt.u8 {
	return self;
}

export function eq$PartialEq(self: Nibble, other: Nibble): boolean {
	return (self & 0xf) === (other & 0xf);
}

export function hash$Hash(self: Nibble, state: rt.Hasher): undefined {
	rt.hash8(self & 0xf, state);
	return undefined;
}

export function partialCmp$PartialOrd(self: Nibble, other: Nibble): rt.Ordering {
	return cmp$Ord(self, other);
}

export function cmp$Ord(self: Nibble, other: Nibble): rt.Ordering {
	return rt.cmp(self & 0xf, other & 0xf);
}
