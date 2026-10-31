// A `OnceLock` initialized from within its own initialization, or a `LazyLock` or
// `LazyCell` forced from within its function, panics with std's message, which native
// Rust can't show: wasm32's `Once` panics, which traps there, and the host's
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

// A lazy cell's function runs once, so a `force` from within it has no value to give.
const lazyLock: rt.LazyLock<string> = new rt.LazyLock(() => {
	recursive(() => lazyLock.force());
	return "made";
});
assert.equal(lazyLock.force(), "made");
const lazyCell: rt.LazyCell<string> = new rt.LazyCell(() => {
	assert.throws(
		() => lazyCell.force(),
		(e: Error) => e instanceof rt.Panic && e.message === "LazyCell instance has previously been poisoned",
	);
	return "made";
});
assert.equal(lazyCell.force(), "made");
