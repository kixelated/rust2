// `rt.spawn` on a task that keeps waking itself: it yields to a timer queued before it
// once a batch spends its budget of 128 polls, so it cannot starve timers and I/O.
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
