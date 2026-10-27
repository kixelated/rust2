// `raw::skip`, which reads through a raw pointer.

import type * as rt from "../rt.ts";

export function skip(bytes: rt.ByteSlice, at: rt.Place<rt.usize>): void {
	while (at.v < bytes.length && bytes.data[bytes.start + at.v] === 0) at.v++;
}
