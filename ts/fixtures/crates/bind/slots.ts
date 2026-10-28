// `crates_dep::Slots`, which writes into the array it is given.

import * as rt from "../rt.ts";

export type Slots = { base: rt.i16 };

export function withBase(base: rt.i16): Slots {
	return { base };
}

export function fill(self: Slots, out: rt.i16[]): void {
	out[0] = self.base;
	out[1] = rt.i16Sub(self.base, 1 as rt.i16);
}
