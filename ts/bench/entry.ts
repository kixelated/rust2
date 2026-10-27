// One engine's run of the benchmark, bundled by `main.ts` beside the files it loads: in
// Node and Bun it prints the result, and in a browser it posts it back to `main.ts`.

import { Wasm } from "../tests/wasm.ts";
import { measure, type Route } from "./time.ts";
import type { Workload } from "./workloads.ts";

type Workloads = { name: string; op: Workload["op"]; items: number; input: string }[];

export interface Config {
	sampleMs: number;
	rounds: number;
	seed: number;
	workloads: Workloads;
	/** The async handoff's, on the task fixture. */
	handoffs: Workloads;
	/** The name of `--route`'s translations, bundled as `route.js` and `task-route.js`. */
	route?: string;
}

const browser = typeof window !== "undefined";

// Tells a browser's `main.ts`, which fails the run when it stops hearing, what runs next. A
// beacon leaves no response for the page to handle between samples.
const progress = browser ? (step: string) => void navigator.sendBeacon("/progress", step) : () => {};

async function load(name: string): Promise<Uint8Array> {
	progress(`loading ${name}`);
	const url = new URL(name, import.meta.url);
	if (url.protocol === "file:") {
		const fs = await import("node:fs/promises");
		return new Uint8Array(await fs.readFile(url));
	}
	return new Uint8Array(await (await fetch(url)).arrayBuffer());
}

/** A bundled route, whose `run` (or `exported`) returns its output as `view` reads it. */
async function route<T>(name: string, file: string, view: (out: T) => Uint8Array, exported = "run"): Promise<Route> {
	progress(`loading ${file}`);
	const run = (await import(new URL(file, import.meta.url).href))[exported];
	return { name, run, output: async (input) => view(await run(input)) };
}

function engine(): string {
	if (browser) return navigator.userAgent;
	const bun = (globalThis as { Bun?: { version: string } }).Bun;
	if (bun) return `Bun ${bun.version} (JavaScriptCore ${process.versions.webkit})`;
	return `Node ${process.version} (V8 ${process.versions.v8})`;
}

function parse(workloads: Workloads): Workload[] {
	return workloads.map((w) => ({ ...w, input: Uint8Array.from(w.input.match(/../g) ?? [], (b) => parseInt(b, 16)) }));
}

async function main() {
	// Without isolation, a browser coarsens performance.now() to as much as a millisecond.
	if (browser && !crossOriginIsolated) throw new Error("the page is not cross-origin isolated");
	const config: Config = JSON.parse(new TextDecoder().decode(await load("workloads.json")));
	const workloads = parse(config.workloads);
	const wasm = new Wasm(await WebAssembly.compile(await load("bench.wasm")));
	// The A/A route is the translation again, loaded as its own module: its spread from the
	// first is the measurement's noise.
	const translated = (out: { data: Uint8Array; length: number }) => out.data.subarray(0, out.length);
	const routes = [
		await route("hand-written", "moq.js", (out: Uint8Array) => out),
		await route("translated", "translated.js", translated),
		await route("A/A", "translated-aa.js", translated),
	];
	if (config.route) routes.push(await route(config.route, "route.js", translated));
	const timings = await measure(wasm, routes, workloads, { ...config, progress });

	// The async handoff: the translation polled at once by a one-task executor, as the
	// native and WASM routes are, and on the runtime's microtask executor.
	const bytes = (out: Uint8Array) => out;
	const handoffRoutes = [
		await route("hand-written", "task-hand.js", bytes, "handWritten"),
		await route("translated", "task-translated.js", bytes),
		await route("spawned", "task-translated.js", bytes, "spawn"),
		await route("A/A", "task-translated-aa.js", bytes),
	];
	if (config.route) handoffRoutes.push(await route(config.route, "task-route.js", bytes));
	const taskWasm = new Wasm(await WebAssembly.compile(await load("task.wasm")));
	timings.push(...(await measure(taskWasm, handoffRoutes, parse(config.handoffs), { ...config, progress })));
	return { engine: engine(), timings };
}

if (browser) {
	const post = (path: string, body: string) => fetch(path, { method: "POST", body });
	main().then(
		(result) => post("/result", JSON.stringify(result)),
		(err: unknown) => post("/error", err instanceof Error ? (err.stack ?? err.message) : String(err)),
	);
} else {
	console.log(JSON.stringify(await main()));
}
