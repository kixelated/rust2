// `just ts bench`: bundles each route, times every workload natively and in Node, Bun,
// and Chrome, and reports the numbers with each route's bundle size.
//
// usage: bun main.ts --translated <dir> --wasm <file> --native <bin> --moq <checkout>
//                    --moq-wasm <dir> --out <dir> [--sample-ms 10] [--rounds 15]

import { cpus, release, totalmem } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import type { BunPlugin } from "bun";

import { Wasm } from "../tests/wasm.ts";
import type { Config } from "./entry.ts";
import { hex, type Timing } from "./time.ts";
import { type Workload, workloads } from "./workloads.ts";

interface Engine {
	engine: string;
	timings: Timing[];
}

const { values: args } = parseArgs({
	options: {
		translated: { type: "string" },
		wasm: { type: "string" },
		native: { type: "string" },
		moq: { type: "string" },
		"moq-wasm": { type: "string" },
		out: { type: "string" },
		"sample-ms": { type: "string", default: "10" },
		rounds: { type: "string", default: "15" },
	},
	strict: true,
});
function arg(name: keyof typeof args): string {
	const value = args[name];
	if (value === undefined) throw new Error(`--${name} is required`);
	return resolve(value);
}
const checkout = arg("moq");
const out = arg("out");
const sampleMs = Number(args["sample-ms"]);
const rounds = Number(args.rounds);

// `moq/` is the checkout, whose own imports resolve through its install.
const moq: BunPlugin = {
	name: "moq",
	setup(build) {
		build.onResolve({ filter: /^moq\// }, ({ path }) => ({ path: join(checkout, path.slice("moq/".length)) }));
	},
};

async function bundle(entry: string, options: { minify?: boolean; plugins?: BunPlugin[] } = {}): Promise<Uint8Array> {
	const result = await Bun.build({ entrypoints: [entry], target: "browser", external: ["node:*"], ...options });
	if (result.outputs.length !== 1) throw new Error(`bundling ${entry} made ${result.outputs.length} files`);
	return new Uint8Array(await result.outputs[0].arrayBuffer());
}

async function read(path: string): Promise<Uint8Array> {
	return new Uint8Array(await Bun.file(path).arrayBuffer());
}

// Chrome's page reports an entry that fails to load or throws before it can report itself.
const page = `<!doctype html>
<script>
	addEventListener("error", (e) => fetch("/error", { method: "POST", body: e.message ?? \`\${e.target.src} failed to load\` }), true);
</script>
<script type="module" src="entry.js"></script>
`;

const handWritten = join(import.meta.dir, "moq.ts");
const translated = join(arg("translated"), "bench.ts");
const wasmBytes = await read(arg("wasm"));
const translation = await bundle(translated);
await Promise.all([
	Bun.write(join(out, "entry.js"), await bundle(join(import.meta.dir, "entry.ts"))),
	Bun.write(join(out, "moq.js"), await bundle(handWritten, { plugins: [moq] })),
	Bun.write(join(out, "translated.js"), translation),
	Bun.write(join(out, "translated-aa.js"), translation),
	Bun.write(join(out, "bench.wasm"), wasmBytes),
	Bun.write(join(out, "package.json"), '{ "type": "module" }\n'),
	Bun.write(join(out, "index.html"), page),
]);

const moqWasm = arg("moq-wasm");
const sizes = [
	{ route: "wasm", files: [wasmBytes] },
	{ route: "hand-written", files: [await bundle(handWritten, { minify: true, plugins: [moq] })] },
	{ route: "translated", files: [await bundle(translated, { minify: true })] },
	{ route: "moq-wasm package", files: [await read(join(moqWasm, "moq_bg.wasm")), await read(join(moqWasm, "moq.js"))] },
].map(({ route, files }) => {
	const sum = (f: (b: Uint8Array) => number) => files.reduce((n, b) => n + f(b), 0);
	return { route, raw: sum((b) => b.byteLength), gzip: sum((b) => Bun.gzipSync(b, { level: 9 }).byteLength) };
});

// The workloads, made by the wasm32 build, which also gives each its expected output.
const wasm = new Wasm(await WebAssembly.compile(wasmBytes));
function expected(input: Uint8Array): Uint8Array {
	const output = wasm.call(input);
	if (output === "panic") throw new Error(`native Rust panics on ${hex(input)}`);
	return output;
}
const list = workloads(expected);
const config: Config = {
	sampleMs,
	rounds,
	workloads: list.map((w) => ({ name: w.name, op: w.op, items: w.items, input: hex(w.input) })),
};
await Bun.write(join(out, "workloads.json"), JSON.stringify(config));

async function native(): Promise<Engine> {
	const stdin = Buffer.from(list.map((w) => `${w.name}/${w.op} ${hex(w.input)}\n`).join(""));
	const proc = Bun.spawn([arg("native"), String(sampleMs), String(rounds)], { stdin, stderr: "inherit" });
	const lines = (await new Response(proc.stdout).text()).trim().split("\n");
	if ((await proc.exited) !== 0) throw new Error("the native benchmark failed");
	const timings = lines.map((line, i): Timing => {
		const [name, calls, ns, output] = line.split(" ");
		const w = list[i];
		if (name !== `${w.name}/${w.op}`) throw new Error(`the native benchmark printed ${name} for ${w.name}/${w.op}`);
		if (output !== hex(expected(w.input))) throw new Error(`native Rust on the host differs from wasm32 on ${name}`);
		return { workload: w.name, op: w.op, route: "native", calls: Number(calls), ns: ns.split(",").map((n) => Number(n) / w.items) };
	});
	return { engine: "native", timings };
}

async function script(command: string): Promise<Engine> {
	const proc = Bun.spawn([command, join(out, "entry.js")], { stderr: "inherit" });
	const result = await new Response(proc.stdout).text();
	if ((await proc.exited) !== 0) throw new Error(`${command} failed`);
	return JSON.parse(result);
}

async function chrome(): Promise<Engine> {
	const { promise, resolve, reject } = Promise.withResolvers<Engine>();
	const server = Bun.serve({
		hostname: "127.0.0.1",
		port: 0,
		async fetch(req) {
			const path = new URL(req.url).pathname;
			if (req.method === "POST") {
				const body = await req.text();
				if (path === "/result") resolve(JSON.parse(body));
				else reject(new Error(`Chrome: ${body}`));
				return new Response();
			}
			const file = Bun.file(join(out, path === "/" ? "index.html" : path.slice(1)));
			if (!(await file.exists())) return new Response(null, { status: 404 });
			// Cross-origin isolated, so performance.now() is precise to 5µs rather than 100.
			const headers = { "Cross-Origin-Opener-Policy": "same-origin", "Cross-Origin-Embedder-Policy": "require-corp" };
			return new Response(file, { headers });
		},
	});
	// No sandbox: the page is our own, and CI runners refuse the user namespaces it needs.
	const proc = Bun.spawn(
		["chromium", "--headless", "--no-sandbox", "--no-first-run", `--user-data-dir=${join(out, "chrome")}`, server.url.href],
		{ stdout: "ignore", stderr: "pipe" },
	);
	proc.exited.then(async (code) => reject(new Error(`Chrome exited with ${code}:\n${await new Response(proc.stderr).text()}`)));
	try {
		// The user agent reduces the version to its major number.
		return { ...(await promise), engine: text(["chromium", "--version"]) };
	} finally {
		proc.kill();
		await server.stop(true);
	}
}

function text(cmd: string[]): string {
	const proc = Bun.spawnSync(cmd, { stderr: "inherit" });
	if (!proc.success) throw new Error(`${cmd.join(" ")} failed`);
	return proc.stdout.toString().trim();
}

const host = await native();
const engines: Engine[] = [];
for (const run of [() => script("node"), () => script(process.execPath), chrome]) engines.push(await run());

const report = render();
await Bun.write(join(out, "report.md"), report);
await Bun.write(join(out, "report.json"), JSON.stringify({ native: host, engines, sizes }, null, "\t"));
console.log(report);

function render(): string {
	const cpu = cpus();
	// Node names no Arm CPU on Linux; lscpu decodes its part number.
	const model =
		process.platform === "linux" ? (text(["lscpu"]).match(/^Model name:\s*(.+)$/m)?.[1] ?? "unknown") : cpu[0].model;
	const lines = [
		"# rust2-ts benchmark",
		"",
		`- Host: ${model}, ${cpu.length} threads, ${(totalmem() / 2 ** 30).toFixed(0)} GiB, ${process.platform} ${release()} ${process.arch}`,
		`- Rust: ${text(["rustc", "-V"])}`,
		`- moq: ${text(["git", "-C", checkout, "rev-parse", "HEAD"])}`,
		...engines.map((e) => `- ${e.engine}`),
		"",
		"Each cell is nanoseconds per item (a varint, a frame header, a message, or a byte iterated): the median",
		`of ${rounds} samples of at least ${sampleMs} ms, ± half their interquartile range. Each round interleaves`,
		"every route and workload. `native` is a release build on the host. `wasm` is the same",
		"Rust at moq-wasm's size-optimized profile, run with its input already in memory;",
		"copying the input in and the output out are timed apart. `hand-written` is moq's",
		"`js/net`, called as its own benchmarks call it, and for `iter/` the same chain with ES",
		"iterator helpers (`bench/iter.ts`). `A/A` is the translation loaded a second time: its",
		"gap from `translated` is the noise.",
		"",
		"## Bundle size",
		"",
		"Bytes for the same workloads, minified by Bun where JS. `moq-wasm package` is what moq",
		"ships as WASM today, its bindgen glue included: the whole consume path, not only the codec.",
		"",
		"| route | raw | gzip |",
		"| --- | ---: | ---: |",
		...sizes.map((s) => `| ${s.route} | ${s.raw} | ${s.gzip} |`),
	];

	const routes = ["native", "wasm", "wasm copy-in", "wasm copy-out", "hand-written", "translated", "A/A"];
	for (const engine of engines) {
		const all = [...host.timings, ...engine.timings];
		const median = (w: Workload, route: string) => stats(find(all, w, route).ns).median;
		const aa = list.map((w) => Math.abs(median(w, "A/A") / median(w, "translated") - 1)).sort((a, b) => a - b);
		lines.push(
			"",
			`## ${engine.engine}`,
			"",
			`A/A differs from translated by ${percent(aa[aa.length >> 1])} at the median workload and ${percent(aa[aa.length - 1])} at most.`,
			"",
			`| workload | op | ${routes.join(" | ")} | translated ÷ hand-written |`,
			`| --- | --- |${" ---: |".repeat(routes.length + 1)}`,
			...list.map((w) => {
				const cells = routes.map((r) => cell(find(all, w, r).ns));
				const ratio = (median(w, "translated") / median(w, "hand-written")).toFixed(2);
				return `| ${w.name} | ${w.op} | ${cells.join(" | ")} | ${ratio} |`;
			}),
		);
	}
	return `${lines.join("\n")}\n`;
}

function find(timings: Timing[], w: Workload, route: string): Timing {
	const timing = timings.find((t) => t.workload === w.name && t.op === w.op && t.route === route);
	if (!timing) throw new Error(`no ${route} timing for ${w.name} ${w.op}`);
	return timing;
}

function stats(ns: number[]) {
	const sorted = [...ns].sort((a, b) => a - b);
	const at = (q: number) => sorted[Math.round(q * (sorted.length - 1))];
	return { median: at(0.5), spread: (at(0.75) - at(0.25)) / 2 / at(0.5) };
}

function cell(ns: number[]): string {
	const { median, spread } = stats(ns);
	return `${Number(median.toPrecision(3))} ±${percent(spread)}`;
}

function percent(x: number): string {
	return `${(x * 100).toFixed(x < 0.1 ? 1 : 0)}%`;
}
