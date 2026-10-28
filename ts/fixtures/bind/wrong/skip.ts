// `raw::skip`, bound wrong: it returns the new position, where Rust returns `()`.

import type * as rt from "../rt.ts";

export function skip(bytes: rt.ByteSlice, at: rt.Place<rt.usize>): rt.usize {
	while (at.v < bytes.length && bytes.data[bytes.start + at.v] === 0) at.v++;
	return at.v;
}
