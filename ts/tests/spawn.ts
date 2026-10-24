// `rt.spawn` on futures that misbehave: one that keeps waking itself yields to a timer
// queued before it once a batch spends its budget of 128 polls, so it cannot starve
// timers and I/O, and one that panics as it drops rejects its promise.
//
// usage: node spawn.ts

import * as rt from "../runtime/rt.ts";

let polls = 0;
let beforeTimer: number | undefined;
const selfWaking: rt.Future<number> = {
	poll(cx: rt.Context): rt.Poll<number> {
		if (++polls === 1000) return polls;
		cx.wakeByRef();
		return rt.Pending;
	},
	[Symbol.dispose](): void {},
};
setTimeout(() => {
	beforeTimer = polls;
}, 0);
await rt.spawn(selfWaking);
if (beforeTimer !== 128) throw new Error(`${beforeTimer} polls ran before the timer, not 128`);

const panicsOnDrop: rt.Future<number> = {
	poll: () => 1,
	[Symbol.dispose]: () => rt.panic("drop failed"),
};
const settled = await rt.spawn(panicsOnDrop).then(
	() => "resolved",
	(err: unknown) => (err instanceof rt.Panic ? err.message : `threw ${err}`),
);
if (settled !== "drop failed") throw new Error(`a panic in drop settled the promise as ${settled}`);
