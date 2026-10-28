// A `OnceLock` initialized from within its own initialization panics with std's message,
// which native Rust can't show: wasm32's `Once` panics, which traps there, and the host's
// deadlocks.
//
// usage: node once.ts

import { strict as assert } from "node:assert";

import * as rt from "../runtime/rt.ts";

const recursive = (f: () => void) => assert.throws(f, (e: Error) => e instanceof rt.Panic && e.message === "one-time initialization may not be performed recursively");

const lock = new rt.OnceLock<string>(undefined);
const made = rt.onceGetOrInit(lock, () => {
	recursive(() => rt.onceSet(lock, "set"));
	recursive(() => rt.onceGetOrInit(lock, () => "inner"));
	return "outer";
});
assert.equal(made, "outer");

// Once initialized, neither panics, and `set` gives its value back.
assert.deepEqual(rt.onceSet(lock, "again"), rt.Err("again"));
