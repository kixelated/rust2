// Times every workload on every route in one engine, after checking that each route's
// output matches the wasm32 build's, which is native Rust's. The WASM route is timed in
// three parts: copying the input in, running, and copying the output out.
//
// Each route runs alone in a realm of its own, a worker, which `measure` drives one
// sample at a time. Routes in one realm share its state: JavaScriptCore merges the type
// profiles of modules with the same text, a module two routes call sees both routes'
// types, and V8 ran a route 2-12% slower loaded after an identical copy than before it.

import type { Wasm } from "../tests/wasm.ts";
import type { Workload } from "./workloads.ts";

/** What one realm times: a route's bundle, or the wasm32 build's three steps. */
export type Subject =
	| {
			route: string;
			/** The bundle, beside `entry.js`. */
			file: string;
			/** The export that runs a workload. */
			exported: string;
			/** Whether `exported` returns the translation's `{ data, length }` rather than bytes. */
			slice: boolean;
	  }
	| { wasm: string };

/** What a realm is told first: what to load, and where later requests arrive. */
export interface Load {
	subject: Subject;
	workloads: Workload[];
	sampleMs: number;
	/**
	 * Each later request: a count `measure` bumps, the op, and its argument. The realm
	 * blocks on it in `Atomics.wait` rather than going back to its event loop, where the
	 * engine's idle-time work spread Firefox's samples of one body up to 2.5x apart.
	 */
	control: Int32Array;
}

/** A realm's replies: the bodies it loaded, then one per request. */
export type Reply = { value: unknown } | { error: string };

/** A worker, as `measure` drives it. */
export interface Link {
	post(load: Load): void;
	/** The next reply, rejected if the worker fails. */
	reply(): Promise<unknown>;
	close(): void;
}

const enum Op {
	/** The route's output on a workload, in hex. */
	Output,
	/** Warm a body up, answering with its calls per sample. */
	Warm,
	/** One sample of a body, in milliseconds. */
	Sample,
}

export interface Options {
	/** How long one sample runs at least, in milliseconds. */
	sampleMs: number;
	/** Samples per timed body, interleaved with every other body's. */
	rounds: number;
	/** Seeds the order of the bodies in each round. */
	seed: number;
	/** Told what runs next, between timed regions. */
	progress(step: string): void;
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
}

// Holds the last result, so no engine can drop a call as dead.
let sink: unknown;

export function hex(bytes: Uint8Array): string {
	return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Times `workloads` on each subject, each in a worker `spawn` starts for it alone. */
export async function measure(
	wasm: Wasm,
	subjects: Subject[],
	workloads: Workload[],
	spawn: () => Link,
	options: Options,
): Promise<Timing[]> {
	const { progress, sampleMs } = options;
	const links: Link[] = [];
	try {
		type Realm = (op: Op, arg: number) => Promise<unknown>;
		const realms: Realm[] = [];
		const bodies: { realm: Realm; index: number; workload: Workload; route: string; calls: number; ns: number[] }[] = [];
		for (const subject of subjects) {
			progress(`loading ${"wasm" in subject ? subject.wasm : subject.file}`);
			const link = spawn();
			links.push(link);
			const control = new Int32Array(new SharedArrayBuffer(12));
			link.post({ subject, workloads, sampleMs, control });
			const list = (await link.reply()) as { route: string; workload: number }[];
			const realm = (op: Op, arg: number) => {
				control[1] = op;
				control[2] = arg;
				Atomics.add(control, 0, 1);
				Atomics.notify(control, 0);
				return link.reply();
			};
			realms.push(realm);
			list.forEach(({ route, workload }, index) => {
				bodies.push({ realm, index, workload: workloads[workload], route, calls: 0, ns: [] });
			});
		}

		for (const [i, workload] of workloads.entries()) {
			const want = wasm.call(workload.input);
			if (want === "panic") throw new Error(`${workload.name} ${workload.op} panics in native Rust`);
			for (const [r, subject] of subjects.entries()) {
				if (!("route" in subject)) continue;
				progress(`checking ${subject.route} on ${workload.name} ${workload.op}`);
				const got = await realms[r](Op.Output, i);
				if (got !== hex(want)) {
					throw new Error(`${subject.route} differs from native Rust on ${workload.name} ${workload.op}:\nnative ${hex(want)}\ngot    ${got}`);
				}
			}
		}

		for (const body of bodies) {
			progress(`warming up ${body.route} on ${body.workload.name} ${body.workload.op}`);
			body.calls = (await body.realm(Op.Warm, body.index)) as number;
		}

		// Each round runs the bodies in a new random order, so drift spreads over all of them
		// and no body keeps a slot: a fixed order runs identical code slower in one slot than
		// in another. Progress is per round: a report per sample slows Chrome's samples by
		// about 10%.
		const span = `${workloads[0].name} through ${workloads[workloads.length - 1].name}`;
		const random = prng(options.seed);
		const order = [...bodies];
		for (let round = 0; round < options.rounds; round++) {
			progress(`timing round ${round + 1} of ${options.rounds} on ${span}`);
			shuffle(order, random);
			for (const body of order) {
				const ms = (await body.realm(Op.Sample, body.index)) as number;
				body.ns.push((ms * 1e6) / (body.calls * body.workload.items));
			}
		}

		return bodies.map(({ workload, route, calls, ns }) => ({ workload: workload.name, op: workload.op, route, calls, ns }));
	} finally {
		for (const link of links) link.close();
	}
}

/**
 * A realm's side of `measure`: loads `load.subject`, importing a route's bundle with
 * `bundle` and instantiating the wasm32 build with `wasm`, then answers each request
 * through `reply` until the worker is closed.
 */
export async function serve(
	load: Load,
	reply: (reply: Reply) => void,
	bundle: (file: string) => Promise<Record<string, unknown>>,
	wasm: (file: string) => Promise<Wasm>,
): Promise<void> {
	const { subject, workloads, sampleMs, control } = load;
	const bodies: Body[] = [];
	const body = (workload: Workload, route: string, call: () => unknown, setup?: () => void) =>
		bodies.push({ workload, route, setup, call, async: false, calls: 1 });
	let output: ((workload: number) => Promise<Uint8Array>) | undefined;
	try {
		if ("wasm" in subject) {
			const instance = await wasm(subject.wasm);
			for (const workload of workloads) {
				const { input } = workload;
				let len = 0;
				body(workload, "wasm", () => instance.run(), () => instance.input(input));
				body(workload, "wasm copy-in", () => instance.input(input));
				body(workload, "wasm copy-out", () => instance.output(len), () => {
					instance.input(input);
					len = instance.run();
				});
			}
		} else {
			const run = (await bundle(subject.file))[subject.exported] as (input: Uint8Array) => unknown;
			const view = subject.slice
				? (out: { data: Uint8Array; length: number }) => out.data.subarray(0, out.length)
				: (out: Uint8Array) => out;
			output = async (i) => view((await run(workloads[i].input)) as never);
			for (const workload of workloads) body(workload, subject.route, () => run(workload.input));
		}
	} catch (err) {
		return reply(failure(err));
	}
	reply({ value: bodies.map((b) => ({ route: b.route, workload: workloads.indexOf(b.workload) })) });

	async function answer(op: Op, arg: number): Promise<unknown> {
		switch (op) {
			case Op.Output:
				if (!output) throw new Error("the wasm32 build is the reference, which has no output to check");
				return hex(await output(arg));
			case Op.Warm: {
				const body = bodies[arg];
				body.setup?.();
				const first = body.call();
				body.async = first instanceof Promise;
				await first;
				while ((await sample(body)) < sampleMs) body.calls *= 2;
				return body.calls;
			}
			case Op.Sample:
				return sample(bodies[arg]);
		}
	}

	// Each wait blocks the worker in this loop, and each `await` goes back only to the
	// microtask queue, which is all an async body needs: the event loop never runs again.
	for (let seen = 0; ; ) {
		Atomics.wait(control, 0, seen);
		seen = Atomics.load(control, 0);
		try {
			reply({ value: await answer(control[1], control[2]) });
		} catch (err) {
			reply(failure(err));
		}
	}
}

function failure(err: unknown): Reply {
	return { error: err instanceof Error ? (err.stack ?? err.message) : String(err) };
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

/** Shuffles `items` in place: Fisher-Yates. */
function shuffle<T>(items: T[], random: () => number) {
	for (let i = items.length - 1; i > 0; i--) {
		const j = Math.floor(random() * (i + 1));
		[items[i], items[j]] = [items[j], items[i]];
	}
}

/** Uniform numbers in [0, 1) from a 32-bit seed: mulberry32. */
function prng(seed: number): () => number {
	return () => {
		seed = (seed + 0x6d2b79f5) | 0;
		let t = Math.imul(seed ^ (seed >>> 15), seed | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 2 ** 32;
	};
}
