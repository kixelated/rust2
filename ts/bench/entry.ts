// One engine's run of the benchmark, bundled by `main.ts` beside the files it loads: in
// Node and Bun it prints the result, and in Chrome it posts it back to `main.ts`.

import { Wasm } from "../tests/wasm.ts";
import { measure, type Route } from "./time.ts";
import type { Workload } from "./workloads.ts";

export interface Config {
	sampleMs: number;
	rounds: number;
	workloads: { name: string; op: Workload["op"]; items: number; input: string }[];
}

const browser = typeof window !== "undefined";

// Tells Chrome's `main.ts`, which fails the run when it stops hearing, what runs next. A
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

/** A bundled route, whose `run` returns its output as `view` reads it. */
async function route<T>(name: string, file: string, view: (out: T) => Uint8Array): Promise<Route> {
	progress(`loading ${file}`);
	const { run } = await import(new URL(file, import.meta.url).href);
	return { name, run, output: async (input) => view(await run(input)) };
}

function engine(): string {
	if (browser) return navigator.userAgent;
	const bun = (globalThis as { Bun?: { version: string } }).Bun;
	if (bun) return `Bun ${bun.version} (JavaScriptCore ${process.versions.webkit})`;
	return `Node ${process.version} (V8 ${process.versions.v8})`;
}

async function main() {
	const config: Config = JSON.parse(new TextDecoder().decode(await load("workloads.json")));
	const workloads = config.workloads.map((w) => ({ ...w, input: Uint8Array.from(w.input.match(/../g) ?? [], (b) => parseInt(b, 16)) }));
	const wasm = new Wasm(await WebAssembly.compile(await load("bench.wasm")));
	// The A/A route is the translation again, loaded as its own module: its spread from the
	// first is the measurement's noise.
	const translated = (out: { view(): Uint8Array }) => out.view();
	const routes = [
		await route("hand-written", "moq.js", (out: Uint8Array) => out),
		await route("translated", "translated.js", translated),
		await route("A/A", "translated-aa.js", translated),
	];
	const timings = await measure(wasm, routes, workloads, { ...config, progress });
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
