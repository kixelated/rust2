// `raw::Tally`, bound wrong: it drops, but has no `[Symbol.dispose]`.

import type * as rt from "../rt.ts";

export interface Tally {
	count: rt.Arc<rt.u32>;
}

function make(count: rt.Arc<rt.u32>): Tally {
	return { count };
}
export { make as new };

export function get(self: Tally): rt.u32 {
	return self.count.value;
}
