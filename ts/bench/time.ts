// Times every workload on every route in one engine, after checking that each route's
// output matches the wasm32 build's, which is native Rust's. The WASM route is timed in
// three parts: copying the input in, running, and copying the output out.

import type { Wasm } from "../tests/wasm.ts";
import type { Workload } from "./workloads.ts";

export interface Route {
	name: string;
	/** Run a workload, as timed: synchronously, or as a promise. */
	run(input: Uint8Array): unknown;
	/** Run a workload for its output. */
	output(input: Uint8Array): Promise<Uint8Array>;
}

export interface Options {
	/** How long one sample runs at least, in milliseconds. */
	sampleMs: number;
	/** Samples per timed body, interleaved with every other body's. */
	rounds: number;
}

export interface Timing {
	workload: string;
	op: string;
	route: string;
	/** Calls per sample. */
	calls: number;
	/** Nanoseconds per item, one per round. */
	ns: number[];
}

interface Body {
	workload: Workload;
	route: string;
	/** Run before each sample, untimed. */
	setup?: () => void;
	call: () => unknown;
	async: boolean;
	calls: number;
	ns: number[];
}

// Holds the last result, so no engine can drop a call as dead.
let sink: unknown;

export function hex(bytes: Uint8Array): string {
	return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

export async function measure(wasm: Wasm, routes: Route[], workloads: Workload[], options: Options): Promise<Timing[]> {
	const bodies: Body[] = [];
	for (const workload of workloads) {
		const { input } = workload;
		const want = wasm.call(input);
		if (want === "panic") throw new Error(`${workload.name} ${workload.op} panics in native Rust`);
		for (const route of routes) {
			const got = hex(await route.output(input));
			if (got !== hex(want)) {
				throw new Error(`${route.name} differs from native Rust on ${workload.name} ${workload.op}:\nnative ${hex(want)}\ngot    ${got}`);
			}
		}

		let len = 0;
		const body = (route: string, call: () => unknown, setup?: () => void) =>
			bodies.push({ workload, route, setup, call, async: false, calls: 1, ns: [] });
		body("wasm", () => wasm.run(), () => wasm.input(input));
		body("wasm copy-in", () => wasm.input(input));
		body("wasm copy-out", () => wasm.output(len), () => {
			wasm.input(input);
			len = wasm.run();
		});
		for (const route of routes) body(route.name, () => route.run(input));
	}

	// Find each body's calls per sample, which also warms it up.
	for (const body of bodies) {
		body.setup?.();
		const first = body.call();
		body.async = first instanceof Promise;
		await first;
		while ((await sample(body)) < options.sampleMs) body.calls *= 2;
	}

	// Interleave the bodies in each round, starting each round one later, so drift and
	// position spread over all of them.
	for (let round = 0; round < options.rounds; round++) {
		for (let i = 0; i < bodies.length; i++) {
			const body = bodies[(i + round) % bodies.length];
			const ms = await sample(body);
			body.ns.push((ms * 1e6) / (body.calls * body.workload.items));
		}
	}

	return bodies.map(({ workload, route, calls, ns }) => ({ workload: workload.name, op: workload.op, route, calls, ns }));
}

/** Milliseconds for one sample of `body.calls` calls. */
async function sample(body: Body): Promise<number> {
	body.setup?.();
	const { call, calls } = body;
	const start = performance.now();
	if (body.async) {
		for (let i = 0; i < calls; i++) sink = await call();
	} else {
		for (let i = 0; i < calls; i++) sink = call();
	}
	return performance.now() - start;
}
