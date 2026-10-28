// What a `OnceLock` initialized from within its own initialization drops, which native
// Rust can't show: wasm32's `Once` panics, which traps there, and the host's deadlocks.
// The call drops what it took, an uncalled function or a value, as std's unwinding does.
//
// usage: node once.ts

import { strict as assert } from "node:assert";

import * as rt from "../runtime/rt.ts";

const dropped: string[] = [];
const recursive = (f: () => void) => assert.throws(f, (e: Error) => e instanceof rt.Panic && e.message === "one-time initialization may not be performed recursively");

const lock = new rt.OnceLock<string>(undefined);
const made = rt.onceGetOrInit(lock, () => {
	recursive(() => rt.onceSet(lock, "set", (v) => dropped.push(v)));
	recursive(() => rt.onceGetOrInit(lock, () => "inner", undefined, () => dropped.push("f")));
	return "outer";
});
assert.equal(made, "outer");
assert.deepEqual(dropped, ["set", "f"]);

// Once initialized, neither panics, and the value `set` refuses comes back undropped.
const refused = rt.onceSet(lock, "again", (v) => dropped.push(v));
assert.deepEqual(refused, rt.Err("again"));
assert.deepEqual(dropped, ["set", "f"]);
