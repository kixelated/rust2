// Runs a fixture's harness both ways on the same inputs and fails on any difference:
// natively, as the fixture's wasm32 build, and as its TypeScript translation.
//
// usage: node differential.ts <fixture.wasm> <harness.ts> <cases>
//
// A harness exports `run(input: Uint8Array): Vec<u8>`; the wasm build exports `input`,
// `run`, and `output` around it. A panic traps natively and throws `Panic` in
// TypeScript, and both count as the same outcome.

import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

type Outcome = Uint8Array | "panic";

interface Exports {
	memory: WebAssembly.Memory;
	input(len: number): number;
	run(): number;
	output(): number;
}

const [wasmPath, harnessPath, countArg] = process.argv.slice(2);
if (!wasmPath || !harnessPath) throw new Error("usage: differential.ts <fixture.wasm> <harness.ts> [cases]");
const count = Number(countArg ?? "20000");

const harness = await import(pathToFileURL(harnessPath).href);
const module = await WebAssembly.compile(await readFile(wasmPath));
let instance = await WebAssembly.instantiate(module, {});

function native(input: Uint8Array): Outcome {
	const exports = instance.exports as unknown as Exports;
	try {
		const ptr = exports.input(input.length);
		new Uint8Array(exports.memory.buffer, ptr, input.length).set(input);
		const len = exports.run();
		const out = exports.output();
		return new Uint8Array(exports.memory.buffer, out, len).slice();
	} catch (err) {
		if (!(err instanceof WebAssembly.RuntimeError)) throw err;
		// A trap leaves the instance mid-call: start a fresh one.
		instance = new WebAssembly.Instance(module, {});
		return "panic";
	}
}

function translated(input: Uint8Array): Outcome {
	try {
		return harness.run(input).view().slice();
	} catch (err) {
		if (err instanceof Error && err.name === "Panic") return "panic";
		throw err;
	}
}

// xorshift64*: the same cases on every run.
let state = 0x9e3779b97f4a7c15n;
function next(): bigint {
	state ^= state >> 12n;
	state ^= (state << 25n) & 0xffffffffffffffffn;
	state ^= state >> 27n;
	return (state * 0x2545f4914f6cdd1dn) & 0xffffffffffffffffn;
}
function byte(): number {
	return Number(next() & 0xffn);
}

/** Bytes biased toward the values codecs care about: zero, all ones, and boundaries. */
function interesting(): number {
	const pick = Number(next() % 8n);
	switch (pick) {
		case 0:
			return 0;
		case 1:
			return 0xff;
		case 2:
			return [0x3f, 0x40, 0x7f, 0x80, 0xbf, 0xc0, 0xdf, 0xe0, 0xef, 0xf0, 0xf7, 0xf8, 0xfb, 0xfc, 0xfd, 0xfe][Number(next() % 16n)];
		default:
			return byte();
	}
}

function hex(outcome: Outcome): string {
	return outcome === "panic" ? "panic" : Buffer.from(outcome).toString("hex");
}

const cases: Uint8Array[] = [new Uint8Array(0)];
// Every op with every short tail, so each path's edges are covered exhaustively.
for (let op = 0; op < 256; op++) {
	cases.push(Uint8Array.of(op));
	for (let b = 0; b < 256; b += 17) cases.push(Uint8Array.of(op, b));
}
// Full-width values through every op: each u64 boundary 2^k - 1, 2^k, 2^k + 1.
for (let op = 0; op < 8; op++) {
	for (let k = 0n; k <= 64n; k++) {
		for (const delta of [-1n, 0n, 1n]) {
			const value = ((1n << k) + delta) & 0xffffffffffffffffn;
			const bytes = new Uint8Array(8);
			new DataView(bytes.buffer).setBigUint64(0, value);
			for (const selector of [0, 1, 2, 3, 4, 5, 6]) {
				cases.push(Uint8Array.of(op, selector, ...bytes, ...bytes, ...bytes));
			}
		}
	}
}
for (let i = 0; i < count; i++) {
	const len = Number(next() % 48n);
	const input = new Uint8Array(len + 1);
	input[0] = byte();
	for (let j = 1; j <= len; j++) input[j] = interesting();
	cases.push(input);
}

let failures = 0;
let panics = 0;
for (const input of cases) {
	const want = native(input);
	let got: Outcome;
	try {
		got = translated(input);
	} catch (err) {
		console.error(`input ${hex(input)}: the translation threw ${err}`);
		failures++;
		continue;
	}
	if (want === "panic") panics++;
	if (hex(want) !== hex(got)) {
		failures++;
		if (failures <= 20) {
			console.error(`input  ${hex(input)}\nnative ${hex(want)}\nts     ${hex(got)}\n`);
		}
	}
}

console.log(`${cases.length} cases, ${panics} panics, ${failures} differences`);
if (failures > 0) process.exit(1);
